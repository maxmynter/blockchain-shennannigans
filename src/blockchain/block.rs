use chrono::Utc;
use hex;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub proof: u64,
}

impl Block {
    pub fn new(index: u64, data: String, proof: u64, previous_hash: String) -> Block {
        let timestamp = Utc::now().timestamp();
        let hash = Block::hash(index, timestamp, &data, &previous_hash, proof);

        Block {
            index,
            data,
            timestamp,
            previous_hash,
            hash,
            proof,
        }
    }

    fn hash(
        index: u64,
        timestamp: i64,
        data: &String,
        previous_hash: &String,
        proof: u64,
    ) -> String {
        let mut hasher = Sha256::new();

        hasher.update(index.to_be_bytes());
        hasher.update(timestamp.to_be_bytes());
        hasher.update(data.as_bytes());
        hasher.update(previous_hash.as_bytes());
        hasher.update(previous_hash.as_bytes());
        hasher.update(proof.to_be_bytes());

        let result = hasher.finalize();

        hex::encode(result)
    }
}
