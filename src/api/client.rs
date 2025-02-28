use reqwest::Client;
use serde_json;

use crate::blockchain::{Block, Chain, Consensus};

// Broadcast a new block to all registered nodes
pub async fn broadcast_block<C: Consensus>(
    chain: &Chain<C>,
    block: &Block<C::Proof>,
) -> Result<(), reqwest::Error> {
    let client = Client::new();
    for node in &chain.nodes {
        client
            .post(&format!("{}/block", node))
            .json(block)
            .send()
            .await?;
    }
    Ok(())
}

// Sync chains with other nodes and adopt the longest valid chain
pub async fn sync_chain<C: Consensus>(chain: &mut Chain<C>) -> Result<(), reqwest::Error> {
    let client = Client::new();
    for node in chain.nodes.clone() {
        let response = client
            .get(&format!("{}/chain", node))
            .send()
            .await?
            .json::<Vec<Block<C::Proof>>>()
            .await?;
        let remote_chain = Chain {
            chain: response,
            nodes: chain.nodes.clone(),
            consensus: chain.consensus.clone(),
        };
        if remote_chain.chain.len() > chain.chain.len() && remote_chain.is_valid() {
            chain.chain = remote_chain.chain;
        }
    }
    Ok(())
}

pub async fn broadcast_node_registration<C: Consensus>(
    chain: &Chain<C>,
    new_node_address: &str,
) -> Result<Vec<String>, reqwest::Error> {
    let client = Client::new();
    let mut successful_broadcasts = Vec::new();

    for node in &chain.nodes {
        if node == new_node_address {
            continue;
        }

        match client
            .post(&format!("{}/nodes/register", node))
            .json(&serde_json::json!({"address": new_node_address}))
            .send()
            .await
        {
            Ok(_) => successful_broadcasts.push(node.clone()),
            Err(e) => {
                println!("Failed to register with node {}:{}", node, e);
            }
        }
    }
    Ok(successful_broadcasts)
}

pub async fn check_node_alive(address: &str) -> bool {
    let client = Client::new();
    let alive_url = format!("{}/alive", address);
    match client.get(&alive_url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}
