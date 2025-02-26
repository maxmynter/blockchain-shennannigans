use super::{Block, Consensus};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "C: Consensus"))]
pub struct Chain<C>
where
    C: Consensus,
    C::Proof: Serialize + for<'b> Deserialize<'b>,
{
    pub chain: Vec<Block<C::Proof>>,
    pub nodes: HashSet<String>,

    pub consensus: C,
}

impl<C> Chain<C>
where
    C: Consensus,
    C::Proof: Serialize + for<'b> Deserialize<'b>,
{
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

    pub fn new_block(&mut self, data: String, timestamp: i64) -> Block<C::Proof> {
        let prev_block = self.chain.last().unwrap();
        let prev_hash = prev_block.hash.clone();

        let proof = self.consensus.prove(self, &data);

        let block = Block::new(self.chain.len() as u64, data, timestamp, proof, prev_hash);
        self.chain.push(block.clone());

        block
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
            if curr.previous_hash != prev.hash || !self.consensus.validate(self, curr) {
                return false;
            }
        }
        return true;
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let chain: Chain<C> = serde_json::from_reader(file)?;
        Ok(chain)
    }

    pub fn load_or_creat(path: &str, consensus: C) -> Self {
        match File::open(path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(chain) => chain,
                Err(_) => Self::new(consensus),
            },
            Err(_) => Self::new(consensus),
        }
    }
}
