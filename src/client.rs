use super::protocol::Socks4IO;

use std::error::Error;
use std::net::Ipv4Addr;

use tokio::net::TcpStream;

pub async fn make_client_tcp_stream(
    socks4_host: Ipv4Addr,
    socks4_port: u16,
    target_host: Ipv4Addr,
    target_port: u16,
) -> Result<TcpStream, Box<dyn Error>> {
    return Socks4IO::make_client_tcp_stream(socks4_host, socks4_port, target_host, target_port)
        .await;
}
