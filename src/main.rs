mod api;
mod blockchain;
mod frontend;
mod utils;

use api::server::run_server;
use blockchain::{Chain, ProofOfWork};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    if let Some(port) = std::env::args().nth(1) {
        let port = port.parse::<u16>().expect("Invalid Port Number");
        let mut chain = Chain::new(ProofOfWork::new(4));
        if port != 8080 {
            chain.add_node(&format!("http://127.0.0.1:8080"));
        }
        if port != 8081 {
            chain.add_node(&format!("http://127.0.0.1:8081"));
        }
        println!("Starting node on port {}", port);

        let address = format!("127.0.0.1:{}", port);
        run_server(chain, &address).await
    } else {
        println!("Please provide port number");
        Ok(())
    }
}
