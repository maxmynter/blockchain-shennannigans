use chrono::Utc;

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
        todo!("Hash this guy");
    }
}
