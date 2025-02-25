use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block<P> {
    pub index: u64,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub proof: P,
}

impl<P: Clone + Serialize + DeserializeOwned> Block<P> {
    pub fn new(
        index: u64,
        data: String,
        timestamp: i64,
        proof: P,
        previous_hash: String,
    ) -> Block<P> {
        let hash = crate::utils::hash(index, timestamp, &data, &previous_hash, &proof);

        Block {
            index,
            data,
            timestamp,
            previous_hash,
            hash,
            proof,
        }
    }
}
