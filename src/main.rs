mod blockchain;

use blockchain::Chain;

fn main() {
    let mut chain = Chain::new();
    println!("{:?}", chain);
    chain.new_block(200, "2nd Block".to_string());
    println!("{:?}", chain);
}
