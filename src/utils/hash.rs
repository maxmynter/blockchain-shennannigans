use hex;
use sha2::{Digest, Sha256};

pub fn hash(
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
    hasher.update(proof.to_be_bytes());

    let result = hasher.finalize();

    hex::encode(result)
}
