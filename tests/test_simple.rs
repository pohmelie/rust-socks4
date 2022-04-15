use std::net::Ipv4Addr;

use socks4::client;
use socks4::server::Server;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time;

async fn one_shot_echo_server(port: u16) {
    let address = format!("{}:{}", "127.0.0.1", port);
    let listener = TcpListener::bind(address).await.unwrap();

    let mut buf: Vec<u8> = [0; 100].to_vec();
    let (mut stream, _) = listener.accept().await.unwrap();
    let n = stream.read(&mut buf).await.unwrap();
    stream.write_all(&buf[..n]).await;
}

#[tokio::test]
async fn test_success_case() {
    tokio::spawn(async {
        one_shot_echo_server(8000).await;
    });
    tokio::spawn(async {
        Server::new("127.0.0.1".to_string(), 1080)
            .serve_forever()
            .await;
    });

    for _ in 1..10 {
        let mut cli = client::make_client_tcp_stream(
            Ipv4Addr::new(127, 0, 0, 1),
            1080,
            Ipv4Addr::new(127, 0, 0, 1),
            8000,
        )
        .await;

        println!("{:?}", cli);

        if cli.is_err() {
            time::sleep(time::Duration::from_millis(100)).await;
            println!("waiting socks server...");
            continue;
        }

        let mut cli = cli.unwrap();

        let mut buf1: Vec<u8> = vec![1, 2, 3, 4];
        cli.write_all(&mut buf1).await.unwrap();
        let mut buf2: Vec<u8> = [0; 4].to_vec();
        cli.read_exact(&mut buf2).await;

        assert_eq!(buf1, buf2);
        break;
    }
}
