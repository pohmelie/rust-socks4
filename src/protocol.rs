use super::structure;
use std::error::Error;
use std::net::Ipv4Addr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TCP_CONNECT_COMMAND_CODE: u8 = 0x01;
const TCP_CONNECT_RESPONSE_OK: u8 = 0x5a;
const TCP_CONNECT_RESPONSE_BAD: u8 = 0x5b;

#[derive(Debug)]
struct Socks4Info {
    version: u8,
    command: u8,
    port: u16,
    ipv4: Vec<u8>,
}

#[derive(Debug)]
pub struct Socks4IO<'a> {
    stream: &'a mut TcpStream,
}

impl<'a> Socks4IO<'a> {
    pub fn new(stream: &'a mut TcpStream) -> Self {
        return Self { stream: stream };
    }

    async fn read_exact(&mut self, size: usize) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buf = vec![0; size];
        self.stream.read_exact(&mut buf).await?;
        return Ok(buf.to_vec());
    }

    async fn read_socks_info(&mut self) -> Result<Socks4Info, Box<dyn Error>> {
        let info = Socks4Info {
            version: self.stream.read_u8().await?,
            command: self.stream.read_u8().await?,
            port: self.stream.read_u16().await?,
            ipv4: self.read_exact(4).await?,
        };
        return Ok(info);
    }

    async fn write_socks_response(&mut self, response: u8) -> Result<(), Box<dyn Error>> {
        let s = structure!("BBH4s");
        let buf: Vec<u8> = s.pack(0, response, 0, &vec![0; 4])?;
        self.stream.write_all(&buf).await?;
        return Ok(());
    }

    async fn read_c_string(&mut self) -> Result<String, Box<dyn Error>> {
        let mut vector = Vec::<u8>::new();
        loop {
            let ch = self.stream.read_u8().await?;
            if ch == 0 {
                break;
            }
            vector.push(ch);
        }
        return Ok(String::from_utf8(vector)?);
    }

    fn ipv4_from_vector(data: &Vec<u8>) -> Ipv4Addr {
        return Ipv4Addr::new(data[0], data[1], data[2], data[3]);
    }

    pub async fn make_server_handshake(mut self) -> Result<(Ipv4Addr, u16), String> {
        let info = self.read_socks_info().await.expect("can't read socks info");

        println!("socks4 info read successfully {:?}", info);
        if info.version != 4 {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD)
                .await
                .expect("info version is not equal to 4");
        }

        if info.command != TCP_CONNECT_COMMAND_CODE {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD)
                .await
                .expect("info action is not equal to tcp_connect");
        }

        self.read_c_string()
            .await
            .expect("can't read closing c-string");

        if info.ipv4.len() != 4 {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD)
                .await
                .expect("info ipv4 length is not equal to 4");
        }

        let resolved_ipv4 = Socks4IO::ipv4_from_vector(&info.ipv4);

        self.write_socks_response(TCP_CONNECT_RESPONSE_OK)
            .await
            .expect("can't write success response to client");

        return Ok((resolved_ipv4, info.port));
    }

    // client part
    pub async fn make_client_tcp_stream(
        socks4_host: Ipv4Addr,
        socks4_port: u16,
        target_host: Ipv4Addr,
        target_port: u16,
    ) -> Result<TcpStream, String> {
        let mut stream = TcpStream::connect((socks4_host, socks4_port))
            .await
            .expect("can't connect to socks");
        let s = structure!("BBH4s");
        let mut buf: Vec<u8> = s
            .pack(
                4,
                TCP_CONNECT_COMMAND_CODE,
                target_port,
                &target_host.octets(),
            )
            .expect("can't pack request");
        buf.push(0); // empty c-string

        stream
            .write_all(&mut buf)
            .await
            .expect("can't wrate request");

        let mut buf: Vec<u8> = [0; 8].to_vec();
        let n = stream
            .read_exact(&mut buf)
            .await
            .expect("can't read response");
        if n != 8 {
            return Err(format!("response lenght is not equal to 8, but {}", n));
        }
        if buf[1] != TCP_CONNECT_RESPONSE_OK {
            return Err(
                "socks server repotrts that connection to remote server can't be established"
                    .to_string(),
            );
        }

        return Ok(stream);
    }
}
