use std::collections::HashSet;

use chrono::Utc;

use super::Block;

pub trait Consensus: Sized {
    fn prove(&self, chain: &Chain<Self>, data: &String) -> u64;
    fn validate(&self, chain: &Chain<Self>, block: &Block) -> bool;
}

#[derive(Debug)]
pub struct Chain<C: Consensus> {
    pub chain: Vec<Block>,
    pub nodes: HashSet<String>,
    consensus: C,
}

impl<C: Consensus> Chain<C> {
    pub fn new(consensus: C) -> Self {
        let mut blockchain = Chain {
            chain: Vec::new(),
            nodes: HashSet::new(),
            consensus,
        };

        // Genesis Block
        let genesis_data = "Fiat Lux".to_string();
        let genesis_proof = blockchain.consensus.prove(&blockchain, &genesis_data);
        let genesis_block = Block::new(
            0,
            genesis_data,
            Utc::now().timestamp(),
            genesis_proof,
            "0".to_string(),
        );
        blockchain.chain.push(genesis_block);

        blockchain
    }

    pub fn new_block(&mut self, data: String, timestamp: i64) -> &Block {
        let prev_block = self.chain.last().unwrap();
        let prev_hash = prev_block.hash.clone();

        let proof = self.consensus.prove(self, &data);

        let block = Block::new(self.chain.len() as u64, data, timestamp, proof, prev_hash);
        self.chain.push(block);

        self.chain.last().unwrap()
    }

    pub fn register_node(&mut self, address: String) {
        self.nodes.insert(address);
    }
    pub fn unregister_node(&mut self, address: &String) {
        self.nodes.remove(address);
    }
    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let prev = &self.chain[i - 1];
            let curr = &self.chain[i];
            if curr.previous_hash != prev.hash || self.consensus.validate(self, curr) {
                return false;
            }
        }
        return true;
    }
}

#[derive(Debug)]
pub struct ProofOfWork {
    difficulty: usize,
}

impl ProofOfWork {
    pub fn new(difficulty: usize) -> Self {
        ProofOfWork { difficulty }
    }
}

impl Consensus for ProofOfWork {
    fn prove(&self, chain: &Chain<Self>, data: &String) -> u64 {
        let previous_hash = if chain.chain.is_empty() {
            "0".to_string() // Genesis Case
        } else {
            chain.chain.last().unwrap().hash.clone()
        };

        let mut proof = 0u64;
        let target = "0".repeat(self.difficulty);
        let timestamp = chrono::Utc::now().timestamp();

        loop {
            let hash = crate::utils::hash(
                chain.chain.len() as u64,
                timestamp,
                &data,
                &previous_hash,
                proof,
            );
            if hash.starts_with(&target) {
                return proof;
            }
            proof += 1;
        }
    }

    fn validate(&self, _chain: &Chain<Self>, block: &Block) -> bool {
        let target = "0".repeat(self.difficulty);
        block.hash.starts_with(&target)
    }
}
