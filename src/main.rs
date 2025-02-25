mod api;
mod blockchain;
mod utils;

use api::server;
use blockchain::{Chain, ProofOfWork};
use chrono::Utc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut chain = Chain::new(ProofOfWork::new(4));

    chain.register_node("http://127.0.0.1:8081".to_string());

    let block = chain.new_block("Ad Astra".to_string(), Utc::now().timestamp());

    if let Err(e) = api::client::broadcast_block(&chain, &block.clone()).await {
        eprintln!("Failed to broadcast block {}", e);
    }

    server::run_server(chain, "127.0.0.1:8080").await?;
    Ok(())
}
