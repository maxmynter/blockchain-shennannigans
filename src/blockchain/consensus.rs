use super::{Block, Chain};
use core::fmt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

pub trait Consensus:
    Sized + Clone + Send + Sync + 'static + Serialize + for<'a> Deserialize<'a>
{
    type Proof: Clone + Serialize + DeserializeOwned + Display;
    fn prove(&self, chain: &Chain<Self>, data: &str) -> Self::Proof;
    fn validate(&self, chain: &Chain<Self>, block: &Block<Self::Proof>) -> bool;
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
                &data,
                &previous_hash,
                &proof,
            );
            if hash.starts_with(&target) {
                return proof;
            }
            proof += 1;
        }
    }

    fn validate(&self, _chain: &Chain<Self>, block: &Block<Self::Proof>) -> bool {
        let target = "0".repeat(self.difficulty);
        block.hash.starts_with(&target)
    }
}
