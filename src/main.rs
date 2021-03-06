use clap::Parser;
use socks4::server::Server;

#[derive(Parser, Debug)]
#[clap(about = "Simple socks4 server", long_about = None)]
struct Args {
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "1080")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let server = Server::new(args.host, args.port);
    server.serve_forever().await.expect("server forever endend");
}
