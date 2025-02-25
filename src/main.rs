mod api;
mod blockchain;
mod utils;

use api::server;
use blockchain::{Chain, ProofOfWork};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    if let Some(port) = std::env::args().nth(1) {
        let port = port.parse::<u16>().expect("Invalid port number");
        let mut chain = Chain::new(ProofOfWork::new(4));
        if port != 8080 {
            chain.register_node(format!("http://127.0.0.1:8080"));
        }
        if port != 8081 {
            chain.register_node(format!("http://127.0.0.1:8081"));
        }
        println!("Starting node on port {}", port);

        server::run_server(chain, &format!("127.0.0.1:{}", port)).await
    } else {
        println!("Please pass port number");
        Ok(())
    }
}
