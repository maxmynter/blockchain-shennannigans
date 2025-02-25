use reqwest::Client;

use crate::blockchain::{Block, Chain, Consensus};

// Broadcast a new block to all registered nodes
pub async fn broadcast_block<C: Consensus>(
    chain: &Chain<C>,
    block: &Block<C::Proof>,
)-> Result<(), reqwest::Error> {
    let client = Client::new();
    for node in &chain.nodes {
        client.post(&format!("{}/block", node)).json(block).send().await?;
    }
    Ok(())
}

// Sync chains with other nodes and adopt the longest valid chain
pub async fn sync_chain<C: Consensus>(chain: &mut Chain<C>) -> Result<(), reqwest::Error>{
    let client = Client::new();
    for node in chain.nodes.clone(){
        let response = client.get(&format!("{}/chain", node)).send().await?.json::<Vec<Block<C::Proof>>>().await?;
        let remote_chain = Chain {
            chain: response, nodes: chain.nodes.clone(), consensus: chain.consensus.clone(),
        };
        if remote_chain.chain.len() > chain.chain.len() && remote_chain.is_valid(){
            chain.chain = remote_chain.chain;
        }
    }
    Ok(())
}
