use super::protocol::Socks4IO;

use std::error::Error;
use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

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

    pub async fn serve_forever(self) -> Result<(), Box<dyn Error>> {
        let address = format!("{}:{}", self.host, self.port);
        println!("trying to listen on {}", address);
        let listener = TcpListener::bind(address).await?;

        loop {
            let (client_socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                Server::handle_connection(client_socket).await;
            });
        }
    }

    async fn handle_connection(client_stream: TcpStream) {
        println!("incoming connection from {:?}", client_stream.peer_addr());
        let (client_stream, ipv4, port) = Server::handshake(client_stream).await.unwrap();
        Server::try_sink(client_stream, ipv4, port).await;
    }

    async fn handshake(mut client_stream: TcpStream) -> Result<(TcpStream, Ipv4Addr, u16), String> {
        let handshake_future = Socks4IO::new(&mut client_stream).make_server_handshake();
        let handshake_timeout_result = timeout(Duration::from_secs(5), handshake_future).await;
        match handshake_timeout_result {
            Ok(handshake_result) => match handshake_result {
                Ok((ipv4, port)) => {
                    return Ok((client_stream, ipv4, port));
                }
                Err(message) => {
                    return Err(message);
                }
            },
            Err(_) => {
                return Err("socks4 handshake timeout elapsed".to_string());
            }
        }
    }

    async fn try_sink(mut client_stream: TcpStream, ipv4: Ipv4Addr, port: u16) {
        match TcpStream::connect((ipv4, port)).await {
            Ok(target_stream) => {
                println!("connected to target {:?}", target_stream.peer_addr());
                let (client_reader, client_writer) = client_stream.into_split();
                let (target_reader, target_writer) = target_stream.into_split();

                println!("starting sink...");
                let h1 = tokio::spawn(async move {
                    Server::_sink("Client".to_string(), client_reader, target_writer).await;
                });
                let h2 = tokio::spawn(async move {
                    Server::_sink("Target".to_string(), target_reader, client_writer).await;
                });

                tokio::join!(h1, h2);
            }
            Err(message) => {
                println!("{}", message);
                client_stream
                    .shutdown()
                    .await
                    .expect("client socket shutdown failed");
            }
        }
    }

    async fn _sink(
        name: String,
        mut reader: OwnedReadHalf,
        mut writer: OwnedWriteHalf,
    ) -> Result<(), Box<dyn Error>> {
        let mut b1 = [0; 8192];
        let mut b2 = [0; 8192];
        loop {
            let peek_result = timeout(Duration::from_secs(30), reader.peek(&mut b1)).await?;
            match peek_result {
                Ok(n) => {
                    if n == 0 {
                        println!("{} peeked {} bytes, closing...", name, n);
                        break;
                    }
                    reader.read(&mut b2).await?;
                    println!("{} read {} bytes, writing...", name, n);
                    writer.write(&b2[..n]).await?;
                }
                Err(_) => {
                    break;
                }
            }
        }
        println!("{} routine done cleanly...", name);
        return Ok(());
    }
}
