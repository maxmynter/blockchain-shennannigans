use std::collections::HashSet;

use super::Block;

pub struct Chain {
    pub chain: Vec<Block>,
    pub nodes: HashSet<String>,
}

impl Chain {
    pub fn new() -> Self {
        let mut blockchain = Chain {
            chain: Vec::new(),
            nodes: HashSet::new(),
        };

        // Genesis Block
        let genesis_block = Block::new(0, "Fiat Lux".to_string(), 100, "0".to_string());
        blockchain.chain.push(genesis_block);

        blockchain
    }

    pub fn new_block(&mut self, proof: u64, data: String) -> &Block {
        let prev_block = self.chain.last().unwrap();
        let prev_hash = prev_block.hash.clone();

        let block = Block::new(self.chain.len() as u64, data, proof, prev_hash);
        self.chain.push(block);

        self.chain.last().unwrap()
    }
    pub fn register_node(&mut self, address: String) {
        self.nodes.insert(address);
    }
}
