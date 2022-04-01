use super::structure;
use std::net::{TcpStream, Ipv4Addr};
use std::io::Write;


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
    stream: &'a TcpStream,

}


impl <'a> Socks4IO<'a> {

    pub fn new(stream: &'a TcpStream) -> Self {
        return Self {
            stream: stream,
        }
    }

    fn read_socks_info(&mut self) -> Socks4Info {
        let s = structure!("BBH4s");
        let tup = s.unpack_from(&mut self.stream).expect("can't decode socks4 header");
        return Socks4Info {
            version: tup.0,
            command: tup.1,
            port: tup.2,
            ipv4: tup.3,
        }
    }

    fn write_socks_response(&mut self, response: u8) {
        let s = structure!("BBH4s");
        let buf: Vec<u8> = s.pack(0, response, 0, &vec![0; 4]).expect("can't encode socks4 header");
        self.stream.write_all(&buf).expect("not all message written to socket");
    }

    fn read_c_string(&mut self) -> String {
        let mut vector = Vec::<u8>::new();
        let s = structure!("s");
        loop {
            let (vec,) = s.unpack_from(&mut self.stream).expect("another c-string char");
            if vec[0] == 0 {
                break;
            }
            vector.push(vec[0]);
        }
        return String::from_utf8(vector).expect("expect utf-8 compatible string");
    }

    fn ipv4_from_vector(&self, data: &Vec<u8>) -> Ipv4Addr {
        return Ipv4Addr::new(
            data[0],
            data[1],
            data[2],
            data[3],
        );
    }

    pub fn evaluate(mut self) -> Result<(Ipv4Addr, u16), String> {
        let info = self.read_socks_info();

        if info.version != 4 {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD);
            return Err("info version is not equal to 4".to_string())
        }

        if info.command != TCP_CONNECT_COMMAND_CODE {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD);
            return Err("info action is not equal to tcp_connect".to_string())
        }

        self.read_c_string();

        if info.ipv4.len() != 4 {
            self.write_socks_response(TCP_CONNECT_RESPONSE_BAD);
            return Err("info ipv4 length is not equal to 4".to_string())
        }

        let resolved_ipv4 = self.ipv4_from_vector(&info.ipv4);

        self.write_socks_response(TCP_CONNECT_RESPONSE_OK);

        return Ok((resolved_ipv4, info.port))
    }

}
