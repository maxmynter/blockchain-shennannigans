use crate::api::client;
use crate::blockchain::{Block, Chain, Consensus, Mempool, MessageTransaction};
use crate::frontend::routes::{
    register_node_form, render_blocks_list, render_dashboard, render_nodes_list,
};
use actix_web::rt::spawn;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;

pub struct AppState {
    pub poll_interval_s: u64,
    pub chain_file: String,
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

#[derive(Deserialize)]
pub struct MessageRequest {
    message: String,
}

pub async fn alive() -> impl Responder {
    HttpResponse::Ok().body("Node alive")
}

// Get /chain: Returns current chain
pub async fn get_chain<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    HttpResponse::Ok().json(chain.chain.clone())
}

// Post /block : Receives a new block and validates it
pub async fn post_block<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    req: HttpRequest,
    block: web::Json<Block<C::Proof>>,
) -> impl Responder
where
    C::Proof: Serialize + Clone + Send + 'static,
    Block<C::Proof>: Serialize + Clone + Send + 'static,
{
    let sender = req
        .connection_info()
        .peer_addr()
        .unwrap_or("unknown")
        .to_string();

    let (is_valid, nodes, block_inner) = {
        let mut chain = data.lock().unwrap();
        let is_valid = chain
            .consensus
            .validate_block(chain.chain.last().unwrap(), &block);

        if is_valid {
            if let Ok(transactions) = serde_json::from_str::<Vec<MessageTransaction>>(&block.data) {
                let transaction_ids: Vec<String> =
                    transactions.iter().map(|tx| tx.id.clone()).collect();
                chain.mempool.remove_messages(&transaction_ids);
            }

            let block_inner = block.into_inner();
            chain.chain.push(block_inner.clone());

            (true, chain.nodes.clone(), block_inner)
        } else {
            (false, HashSet::new(), block.into_inner())
        }
    };
    if is_valid {
        let block_clone = block_inner.clone();
        tokio::spawn(async move {
            if let Err(e) =
                crate::api::client::broadcast_block::<C>(&nodes, &block_clone, Some(sender)).await
            {
                eprintln!("Error propagating block:{}", e)
            }
        });
        HttpResponse::Ok().body("block added")
    } else {
        HttpResponse::BadRequest().body("Invalid block")
    }
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
    spawn(client::broadcast_node_registration(
        chain_clone,
        new_address,
    ));

    HttpResponse::Ok().body(format!("Node {} registered", req.address))
}

pub async fn get_nodes<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let nodes = chain.nodes.clone();
    let registered_nodes = RegisteredNodes { nodes };

    HttpResponse::Ok().json(registered_nodes)
}

async fn synchronize_chain<C: Consensus>(
    chain_data: &web::Data<Mutex<Chain<C>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let nodes = {
        let chain = chain_data.lock().unwrap();
        chain.nodes.clone()
    };
    if nodes.is_empty() {
        return Ok(());
    }
    let mut max_len = 0;
    let mut best_chain: Option<Vec<Block<C::Proof>>> = None;
    for node in nodes {
        match client::sync_chain::<C>(&node).await {
            Ok(response) => {
                let temp_chain = Chain {
                    chain: response,
                    nodes: Default::default(),
                    consensus: chain_data.lock().unwrap().consensus.clone(),
                    mempool: Mempool::new(10, 100),
                };
                if temp_chain.consensus.validate_chain(&temp_chain)
                    && temp_chain.chain.len() > max_len
                {
                    max_len = temp_chain.chain.len();
                    best_chain = Some(temp_chain.chain);
                }
            }
            Err(_) => continue,
        }
    }

    if let Some(new_chain) = best_chain {
        let mut chain = chain_data.lock().unwrap();
        if new_chain.len() > chain.chain.len() {
            chain.chain = new_chain;
            println!("Chain updated. New length {}", max_len);
        }
    }
    Ok(())
}

pub async fn submit_message<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    req: web::Json<MessageRequest>,
) -> impl Responder {
    let mut chain = data.lock().unwrap();
    match chain.submit_message(req.message.clone()) {
        Ok(transaction) => HttpResponse::Ok().json(transaction),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub async fn generate_block<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder
where
    C::Proof: Serialize,
    Block<C::Proof>: Serialize,
{
    let (block_option, nodes) = {
        let mut chain = data.lock().unwrap();
        let timestamp = chrono::Utc::now().timestamp();
        let block = chain.new_block(timestamp, 10).await;
        let nodes = if block.is_some() {
            chain.nodes.clone()
        } else {
            HashSet::new()
        };
        (block, nodes)
    };

    match block_option {
        Some(block) => {
            let block_clone = block.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    crate::api::client::broadcast_block::<C>(&nodes, &block_clone, None).await
                {
                    eprintln!("Error broadcasting new block{}", e);
                }
            });
            HttpResponse::Ok().json(block)
        }
        None => HttpResponse::BadRequest().body("No pending transactions in mempool"),
    }
}

pub async fn get_pending_transactions<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
) -> impl Responder {
    let chain = data.lock().unwrap();

    let pending_count = chain.mempool.pending_count();

    #[derive(Serialize)]
    struct MempoolStatus {
        pending_transactions: usize,
    }

    HttpResponse::Ok().json(MempoolStatus {
        pending_transactions: pending_count,
    })
}

fn configure_api_routes<C: Consensus>(cfg: &mut web::ServiceConfig) {
    cfg.route("/chain", web::get().to(get_chain::<C>))
        .route("/block", web::post().to(post_block::<C>))
        .route("/generate", web::post().to(generate_block::<C>))
        .route("/submit", web::post().to(submit_message::<C>))
        .route("/pending", web::get().to(get_pending_transactions::<C>))
        .route("/nodes", web::get().to(get_nodes::<C>))
        .route("/nodes/register", web::post().to(register_node::<C>))
        .route("/alive", web::get().to(alive));
}

fn configure_frontend_routes<C: Consensus>(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(render_dashboard::<C>))
        .route("/message", web::post().to(submit_message::<C>))
        .route("/web/nodes", web::get().to(render_nodes_list::<C>))
        .route(
            "/web/nodes/register",
            web::post().to(register_node_form::<C>),
        )
        .route("/web/nodes/list", web::get().to(render_nodes_list::<C>))
        .route("/web/blocks/list", web::get().to(render_blocks_list::<C>));
}

// Start server with given chain and address
pub async fn run_server<C: Consensus>(
    chain: Chain<C>,
    address: &str,
    chain_file: String,
) -> std::io::Result<()>
where
    C::Proof: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let chain_data = web::Data::new(Mutex::new(chain));
    println!("Starting rustchain node on port {}", address);

    let consensus_data = chain_data.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = synchronize_chain(&consensus_data).await {
                eprintln!("Error synchronizing chain: {}", e);
            }
        }
    });

    let persistence_data = chain_data.clone();
    let chain_file_clone = chain_file.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Ok(chain) = persistence_data.lock() {
                if let Err(e) = chain.save_to_file(&chain_file_clone) {
                    eprintln!("Error saving chain: {}", e);
                } else {
                    println!("Chain saved to {}", chain_file_clone);
                }
            }
        }
    });

    let app_state = web::Data::new(AppState {
        poll_interval_s: super::POLL_INTERVAL_S,
        chain_file,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(chain_data.clone())
            .app_data(app_state.clone())
            .configure(configure_api_routes::<C>)
            .configure(configure_frontend_routes::<C>)
    })
    .bind(address)?
    .run()
    .await
}
