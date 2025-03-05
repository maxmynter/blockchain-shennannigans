use super::{Block, Consensus, Mempool, MessageTransaction};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound(deserialize = "C: Consensus"))]
pub struct Chain<C>
where
    C: Consensus,
    C::Proof: Serialize + for<'b> Deserialize<'b>,
{
    pub chain: Vec<Block<C::Proof>>,
    pub nodes: HashSet<String>,

    pub consensus: C,
    pub mempool: Mempool,
}

impl<C> Chain<C>
where
    C: Consensus,
    C::Proof: Serialize + for<'b> Deserialize<'b>,
{
    pub async fn new(consensus: C) -> Self {
        let mut blockchain = Chain {
            chain: Vec::new(),
            nodes: HashSet::new(),
            consensus,
            mempool: Mempool::new(2, 100),
        };

        // Genesis Block
        let genesis_data = "Fiat Lux".to_string();
        let genesis_proof = blockchain.consensus.prove(&blockchain, &genesis_data).await;
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

    pub async fn new_block(&mut self, data: String, timestamp: i64) -> Block<C::Proof> {
        // TODO: Remove in favour of new_block_from_mempool
        // Figure out how to render individual messages on the frontend.
        let prev_block = self.chain.last().unwrap();
        let prev_hash = prev_block.hash.clone();

        let proof = self.consensus.prove(self, &data).await;

        let block = Block::new(self.chain.len() as u64, data, timestamp, proof, prev_hash);
        self.chain.push(block.clone());

        block
    }

    pub fn submit_message(&mut self, message: String) -> Result<MessageTransaction, String> {
        self.mempool.add_message(message)
    }

    pub async fn new_block_from_mempool(
        &mut self,
        timestamp: i64,
        max_messages: usize,
    ) -> Option<Block<C::Proof>> {
        let messages = self.mempool.get_pending_messages(max_messages);
        if messages.is_empty() {
            return None;
        }

        let data = serde_json::to_string(&messages).unwrap_or_default();
        let prev_block = self.chain.last().unwrap();
        let prev_hash = prev_block.hash.clone();
        let proof = self.consensus.prove(self, &data).await;
        let block = Block::new(self.chain.len() as u64, data, timestamp, proof, prev_hash);

        let message_ids: Vec<String> = messages.iter().map(|tx| tx.id.clone()).collect();
        self.mempool.remove_messages(&message_ids);
        self.chain.push(block.clone());

        Some(block)
    }

    pub fn add_node(&mut self, address: &str) {
        self.nodes.insert(address.to_owned());
    }
    pub fn remove_node(&mut self, address: &String) {
        self.nodes.remove(address);
    }
    pub fn is_valid(&self) -> bool {
        for i in 1..self.chain.len() {
            let prev = &self.chain[i - 1];
            let curr = &self.chain[i];
            if curr.previous_hash != prev.hash || !self.consensus.validate_block(prev, curr) {
                return false;
            }
        }
        true
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

    pub async fn load_or_create(path: &str, consensus: C) -> Self {
        match File::open(path) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(chain) => chain,
                Err(_) => Self::new(consensus).await,
            },
            Err(_) => Self::new(consensus).await,
        }
    }
}
