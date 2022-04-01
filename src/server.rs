use super::protocol::Socks4IO;

use std::net::{TcpListener, Shutdown, TcpStream, Ipv4Addr};
use std::time::Duration;
use std::thread;
use std::io::Read;
use std::io::Write;


#[derive(Debug)]
pub struct Server {
    pub host: String,
    pub port: u16,
}


impl Server {

    pub fn new(host: String, port: u16) -> Self {
        return Self {
            host: host,
            port: port,
        }
    }

    pub fn serve_forever(self) {
        let address = format!("{}:{}", self.host, self.port);
        println!("trying to listen on {}", address);
        let listener = TcpListener::bind(address).unwrap();

        for stream in listener.incoming() {
            thread::spawn(|| Server::handle_connection(stream.unwrap()));
        }
    }

    fn _configure_stream(stream: &TcpStream) {
        stream.set_read_timeout(Some(Duration::from_secs(30))).expect("set read timeout failed");
        stream.set_write_timeout(Some(Duration::from_secs(30))).expect("set write timeout failed");
    }

    fn handle_connection(client_stream: TcpStream) {
        Server::_configure_stream(&client_stream);
        match Socks4IO::new(&client_stream).evaluate() {
            Ok((ipv4, port)) => Server::try_sink(client_stream, ipv4, port),
            Err(message) => {
                println!("{}", message);
                client_stream.shutdown(Shutdown::Both);
            }
        }
    }

    fn _sink(mut read_stream: &TcpStream, mut write_stream: &TcpStream) {
        let mut buf = [0, 1024];
        loop {
            let count = read_stream.read(&mut buf).unwrap();
            if count == 0 {
                return;
            }
            let count = write_stream.write(&buf[..count]).unwrap();
            if count == 0 {
                return;
            }
        }
    }

    fn try_sink <'a>(client_stream: TcpStream, ipv4: Ipv4Addr, port: u16) {
        match TcpStream::connect((ipv4, port)) {
            Ok(target_stream) => {
                Server::_configure_stream(&target_stream);
                let client_stream: &'a TcpStream = &client_stream;
                let target_stream: &'a TcpStream = &target_stream;
                let h1 = thread::spawn(|| Server::_sink(client_stream, target_stream));
                let h2 = thread::spawn(|| Server::_sink(target_stream, client_stream));

                h1.join().unwrap();
                h2.join().unwrap();

                client_stream.shutdown(Shutdown::Both);
                target_stream.shutdown(Shutdown::Both);
            },
            Err(_) => {
                client_stream.shutdown(Shutdown::Both);
            }
        }
    }
}
