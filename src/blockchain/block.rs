use crate::blockchain::Transaction;

pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transaction: Vec<Transaction>,
    pub proof: u64,
    pub previous_hash: String,
}
