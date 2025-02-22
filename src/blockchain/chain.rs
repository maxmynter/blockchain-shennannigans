use crate::blockchain::Block;

pub struct Chain {
    pub chain: Vec<Block>,
}
impl Chain {
    pub fn new() {
        todo!("Not yet implemented");
    }

    pub fn tx(sender: &String, recipient: &String, amount: u64) -> u64 {
        todo!("Transaction not yet implemented");
    }
}
