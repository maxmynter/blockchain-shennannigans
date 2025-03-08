use super::{Block, Chain};
use core::fmt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::future::Future;
use std::pin::Pin;

pub trait Consensus:
    Sized + Clone + Send + Sync + 'static + Serialize + for<'a> Deserialize<'a>
{
    type Proof: Debug + Sync + Clone + Serialize + DeserializeOwned + Display + Send;
    fn prove<'a>(
        &'a self,
        next_index: u64,
        timestamp: i64,
        data: &'a str,
        previous_hash: &'a str,
    ) -> Pin<Box<dyn Future<Output = Self::Proof> + Send + 'a>>;

    fn validate_block(
        &self,
        previous_block: &Block<Self::Proof>,
        block: &Block<Self::Proof>,
    ) -> bool;

    fn validate_chain(&self, chain: &Chain<Self>) -> bool {
        if chain.chain.is_empty() {
            return true;
        }
        let genesis = &chain.chain[0];
        if genesis.index != 0 || genesis.previous_hash != "0" {
            return false;
        }
        for i in 1..chain.chain.len() {
            if !self.validate_block(&chain.chain[i - 1], &chain.chain[i]) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofOfWork {
    difficulty: usize,
}

impl ProofOfWork {
    pub fn new(difficulty: usize) -> Self {
        ProofOfWork { difficulty }
    }
}

impl fmt::Display for ProofOfWork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Difficulty (leading zeros in hash) = {}",
            self.difficulty
        )
    }
}

impl Consensus for ProofOfWork {
    type Proof = u64;

    fn prove<'a>(
        &'a self,
        next_index: u64,
        timestamp: i64,
        data: &'a str,
        previous_hash: &'a str,
    ) -> Pin<Box<dyn Future<Output = Self::Proof> + Send + 'a>> {
        let difficulty = self.difficulty;
        let data_clone = data.to_string();
        let previous_hash_clone = previous_hash.to_string();

        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                let target = "0".repeat(difficulty);
                let mut proof = 0u64;

                loop {
                    let hash = crate::utils::hash(
                        next_index,
                        timestamp,
                        &data_clone,
                        &previous_hash_clone,
                        &proof,
                    );
                    if hash.starts_with(&target) {
                        return proof;
                    }
                    proof += 1;
                }
            })
            .await
            .expect("Mining task failed")
        })
    }

    fn validate_block(
        &self,
        previous_block: &Block<Self::Proof>,
        block: &Block<Self::Proof>,
    ) -> bool {
        if block.index != previous_block.index + 1 {
            return false;
        }

        if block.previous_hash != previous_block.hash {
            return false;
        }

        let calculated_hash = crate::utils::hash(
            block.index,
            block.timestamp,
            &block.data,
            &block.previous_hash,
            &block.proof,
        );
        if block.hash != calculated_hash {
            return false;
        }

        let target = "0".repeat(self.difficulty);
        block.hash.starts_with(&target)
    }
}
