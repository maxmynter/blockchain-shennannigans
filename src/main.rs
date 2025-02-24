mod blockchain;
mod utils;

use blockchain::{Chain, ProofOfWork};

fn main() {
    let pow = ProofOfWork::new(3);

    let mut chain = Chain::new(pow);

    println!("{:?}", chain);
    chain.new_block("2nd Block".to_string());
    println!("{:?}", chain);
}
