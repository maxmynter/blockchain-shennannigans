use std::collections::HashSet;

use reqwest::Client;
use serde_json;

use crate::blockchain::{Block, Chain, Consensus};

// Broadcast a new block to all registered nodes
pub async fn broadcast_block<C: Consensus>(
    nodes: &HashSet<String>,
    block: &Block<C::Proof>,
    origin_node: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    for node in nodes {
        if let Some(ref origin) = origin_node {
            if node == origin {
                continue;
            }
        }
        match client
            .post(format!("{}/block", node))
            .json(block)
            .send()
            .await
        {
            Ok(_) => println!("Block broadcast to {} successful", node),
            Err(e) => eprintln!("Error broadcasting block to {}: {}", node, e),
        }
    }
    Ok(())
}

// Sync chains with other nodes and adopt the longest valid chain
pub async fn sync_chain<C: Consensus>(
    node_address: &str,
) -> Result<Vec<Block<C::Proof>>, reqwest::Error>
where
    C::Proof: serde::de::DeserializeOwned,
{
    let client = Client::new();

    client
        .get(format!("{}/chain", node_address))
        .send()
        .await?
        .json::<Vec<Block<C::Proof>>>()
        .await
}

pub async fn broadcast_node_registration<C: Consensus>(
    chain: Chain<C>,
    new_node_address: String,
) -> Result<Vec<String>, reqwest::Error> {
    let client = Client::new();
    let mut successful_broadcasts = Vec::new();

    for node in chain.nodes {
        if node == new_node_address {
            continue;
        }

        match client
            .post(format!("{}/nodes/register", node))
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
