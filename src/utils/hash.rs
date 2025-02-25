use hex;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn hash<T: Serialize>(
    index: u64,
    timestamp: i64,
    data: &str,
    previous_hash: &str,
    proof: &T,
) -> String {
    let mut hasher = Sha256::new();

    hasher.update(index.to_be_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.to_be_bytes());
    hasher.update(b"|");
    hasher.update(data.as_bytes());
    hasher.update(b"|");
    hasher.update(previous_hash.as_bytes());
    hasher.update(b"|");

    let proof_bytes = serde_json::to_vec(proof).unwrap_or_else(|_| vec![]);
    hasher.update(&proof_bytes);

    let result = hasher.finalize();

    hex::encode(result)
}
