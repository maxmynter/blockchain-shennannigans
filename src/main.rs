mod api;
mod blockchain;
mod frontend;
mod utils;

use api::server::run_server;
use blockchain::{Chain, ProofOfWork};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let consensus_type = "pow"; // TODO: later read that in from .env ...

    if let Some(port) = std::env::args().nth(1) {
        let port = port.parse::<u16>().expect("Invalid Port Number");
        let chain = match consensus_type {
            "pow" => {
                let difficulty = 4;
                Chain::new(ProofOfWork::new(difficulty))
            }
            "pos" => {
                unimplemented!("Proof of stake not implemented")
            }
            _ => panic!("Unsupported consensus type {}", consensus_type),
        };

        let mut chain = chain;
        if port != 8080 {
            chain.add_node("http://127.0.0.1:8080");
        }
        if port != 8081 {
            chain.add_node("http://127.0.0.1:8081");
        }

        println!(
            "Starting node on port {} with consensus {}",
            port, consensus_type
        );

        let address = format!("127.0.0.1:{}", port);
        run_server(chain, &address).await
    } else {
        println!("Please provide port number");
        Ok(())
    }
}
