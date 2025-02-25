use reqwest::Client;
use crate::blockchain::{Chain, Block, ProofOfWork}

pub async fn broadcast_block(chain: &Chain<ProofOfWork>, block: &Block) -> Result<(), reqwest::Error>{
    todo("Not yet implemented")
}
