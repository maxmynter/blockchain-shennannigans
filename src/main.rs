mod blockchain;
mod utils;

use blockchain::{Chain, ProofOfWork};
use chrono::Utc;

fn main() {
    let pow = ProofOfWork::new(3);

    let mut chain = Chain::new(pow);
    let node1 = "http://localhost:8080".to_string();
    chain.register_node(node1.clone());
    chain.unregister_node(&node1);

    println!("{:?}", chain);
    chain.new_block("2nd Block".to_string(), Utc::now().timestamp());
    println!("{:?}", chain);
}
