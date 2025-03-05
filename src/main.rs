mod api;
mod blockchain;
mod frontend;
mod utils;

use api::server::run_server;
use blockchain::{Chain, ProofOfWork};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    port: u16,

    #[arg(short = 'f', long, num_args = 0..=1, default_missing_value = "")]
    chain_file: Option<String>,

    #[arg(short, long, default_value = "pow")]
    consensus: String,

    #[arg(short, long, default_value_t = 4)]
    difficulty: u64,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let chain_file = match args.chain_file {
        Some(file) if !file.is_empty() => file,
        _ => format!("chain_{}.json", args.port),
    };

    let chain = match args.consensus.as_str() {
        "pow" => {
            Chain::load_or_create(&chain_file, ProofOfWork::new(args.difficulty as usize)).await
        }
        "pos" => {
            unimplemented!("Proof of Stake not implemented.")
        }
        _ => panic!("Unsupported Consensus type {}", args.consensus),
    };

    println!(
        "Starting node on port {} with consensus {} (difficulty {}) (chain file: {})",
        args.port, args.consensus, args.difficulty, chain_file
    );
    let chain = chain;

    let address = format!("127.0.0.1:{}", args.port);
    run_server(chain, &address, chain_file).await
}
