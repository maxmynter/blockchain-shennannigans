use super::{Block, Chain};
use core::fmt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait Consensus:
    Sized + Clone + Send + Sync + 'static + Serialize + for<'a> Deserialize<'a>
{
    type Proof: Debug + Sync + Clone + Serialize + DeserializeOwned + Display + Send;
    fn prove(&self, chain: &Chain<Self>, data: &str) -> Self::Proof;
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

    fn prove(&self, chain: &Chain<Self>, data: &str) -> Self::Proof {
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
                data,
                &previous_hash,
                &proof,
            );
            if hash.starts_with(&target) {
                return proof;
            }
            proof += 1;
        }
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
