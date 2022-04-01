use super::protocol::Socks4IO;

use std::io::Read;
use std::io::Write;
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

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
        };
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
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .expect("set write timeout failed");
    }

    fn handle_connection(client_stream: TcpStream) {
        println!("incoming connection from {:?}", client_stream.peer_addr());
        Server::_configure_stream(&client_stream);
        match Socks4IO::new(&client_stream).evaluate() {
            Ok((ipv4, port)) => {
                println!("socks4 handshake ok");
                Server::try_sink(client_stream, ipv4, port);
            }
            Err(message) => {
                println!("{}", message);
                client_stream
                    .shutdown(Shutdown::Both)
                    .expect("client shutdown failed");
            }
        }
    }

    fn _sink<'a>(
        mut read_stream: &'a TcpStream,
        mut write_stream: &'a TcpStream,
    ) -> Result<usize, String> {
        let mut buf: Vec<u8> = vec![0; 1024];
        loop {
            read_stream
                .set_read_timeout(Some(Duration::from_millis(100)))
                .expect("set read timeout failed");
            let result = read_stream.read(&mut buf);
            if result.is_err() {
                return Err("read timeout".to_string());
            }
            let read_count = result.unwrap();
            let write_count = write_stream.write(&buf[..read_count]).unwrap();
            if read_count != write_count {
                println!("{} != {}", read_count, write_count);
            }
            return Ok(read_count);
        }
    }

    fn try_sink(client_stream: TcpStream, ipv4: Ipv4Addr, port: u16) {
        match TcpStream::connect((ipv4, port)) {
            Ok(target_stream) => {
                println!("connected to target {:?}", target_stream.peer_addr());
                Server::_configure_stream(&target_stream);

                loop {
                    let client_count = Server::_sink(&client_stream, &target_stream).unwrap_or(0);
                    println!("client to target {} bytes", client_count);
                    let target_count = Server::_sink(&target_stream, &client_stream).unwrap_or(0);
                    println!("target to client {} bytes", target_count);
                    if client_count + target_count == 0 {
                        break;
                    }
                }

                client_stream
                    .shutdown(Shutdown::Both)
                    .expect("client shutdown failed");
                target_stream
                    .shutdown(Shutdown::Both)
                    .expect("target shutdown faield");
                println!("both connection closed");
            }
            Err(_) => {
                client_stream
                    .shutdown(Shutdown::Both)
                    .expect("target shutdown faield");
                println!("can't connect to target {}:{}", ipv4, port);
            }
        }
    }
}
