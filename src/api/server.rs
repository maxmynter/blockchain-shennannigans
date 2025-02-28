use crate::api::client;
use crate::blockchain::{Block, Chain, Consensus};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct ChainWrapper<P> {
    chain: Vec<Block<P>>,
}

#[derive(Deserialize)]
pub struct BlockRequest {
    data: String,
}

#[derive(Deserialize)]
pub struct NodeRequest {
    address: String,
}

#[derive(Serialize, Deserialize)]
pub struct RegisteredNodes {
    nodes: HashSet<String>,
}

pub async fn alive() -> impl Responder {
    HttpResponse::Ok().body("Node alive")
}

// Get /chain: Returns current chain
pub async fn get_chain<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let wrapper = ChainWrapper {
        chain: chain.chain.clone(),
    };
    HttpResponse::Ok().json(wrapper)
}

// Post /block : Receives a new block and validates it
pub async fn post_block<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    block: web::Json<Block<C::Proof>>,
) -> impl Responder {
    let mut chain = data.lock().unwrap();
    if chain
        .consensus
        .validate_block(&chain.chain.last().unwrap(), &block)
    {
        chain.chain.push(block.into_inner());
        HttpResponse::Ok().body("Block added")
    } else {
        HttpResponse::BadRequest().body("Invalid Block")
    }
}

pub async fn generate_block<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    req: web::Json<BlockRequest>,
) -> impl Responder
where
    C::Proof: Serialize,
    Block<C::Proof>: Serialize,
{
    let mut chain = data.lock().unwrap();
    let timestamp = chrono::Utc::now().timestamp();

    let block = chain.new_block(req.data.clone(), timestamp);

    HttpResponse::Ok().json(block)
}

pub async fn register_node<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    req: web::Json<NodeRequest>,
) -> impl Responder {
    let new_address = req.address.clone();
    if !client::check_node_alive(&new_address).await {
        return HttpResponse::BadRequest().body(format!("Node {} cannot be reached", new_address));
    }
    {
        let mut chain = data.lock().unwrap();
        chain.add_node(&new_address);
    }
    // Clone chain to release mutex and allow concurrency
    let chain_clone = data.lock().unwrap().clone();
    client::broadcast_node_registration(&chain_clone, &new_address);

    HttpResponse::Ok().body(format!("Node {} registered", req.address))
}

pub async fn get_nodes<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let nodes = chain.nodes.clone();
    let registered_nodes = RegisteredNodes { nodes };

    HttpResponse::Ok().json(registered_nodes)
}

// Start server with given chain and address
pub async fn run_server<C: Consensus>(chain: Chain<C>, address: &str) -> std::io::Result<()>
where
    C::Proof: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let chain_data = web::Data::new(Mutex::new(chain));
    HttpServer::new(move || App::new().app_data(chain_data.clone()))
        .bind(address)?
        .run()
        .await
}
