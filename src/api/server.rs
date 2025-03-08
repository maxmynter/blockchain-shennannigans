use crate::api::client;
use crate::blockchain::{
    Block, Chain, ChainInfo, Consensus, Mempool, MessageQueue, MessageTransaction, MiningCommand,
    MiningCoordinator, MiningInterface,
};
use crate::frontend::routes::{
    register_node_form, render_blocks_list, render_dashboard, render_nodes_list,
};
use actix_web::rt::spawn;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

type MiningJoinHandle = tokio::task::JoinHandle<()>;

pub struct AppState<C: Consensus> {
    pub poll_interval_s: u64,
    pub chain_file: String,
    pub mining_tx: Sender<MiningCommand>,
    pub mining_handle: Option<MiningJoinHandle>,
    pub chain_info: Arc<Mutex<ChainInfo>>,
    _consensus_type: std::marker::PhantomData<C>,
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
pub async fn get_chain<C: Consensus>(data: web::Data<Arc<Mutex<Chain<C>>>>) -> impl Responder {
    let chain = data.lock().await;
    HttpResponse::Ok().json(chain.chain.clone())
}

// Post /block : Receives a new block and validates it
pub async fn post_block<C: Consensus>(
    data: web::Data<Arc<Mutex<Chain<C>>>>,
    app_state: web::Data<AppState<C>>,
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
        let mut chain = data.lock().await;
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

            let mut info = app_state.chain_info.lock().await;
            info.length = chain.chain.len() as u64;
            info.last_hash = block_inner.hash.clone();

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
    data: web::Data<Arc<Mutex<Chain<C>>>>,
    req: web::Json<NodeRequest>,
) -> impl Responder {
    let new_address = req.address.clone();
    if !client::check_node_alive(&new_address).await {
        return HttpResponse::BadRequest().body(format!("Node {} cannot be reached", new_address));
    }
    {
        let mut chain = data.lock().await;
        chain.add_node(&new_address);
    }
    // Clone chain to release mutex and allow concurrency
    let chain_clone = data.lock().await.clone();
    spawn(client::broadcast_node_registration(
        chain_clone,
        new_address,
    ));

    HttpResponse::Ok().body(format!("Node {} registered", req.address))
}

pub async fn get_nodes<C: Consensus>(data: web::Data<Arc<Mutex<Chain<C>>>>) -> impl Responder {
    let chain = data.lock().await;
    let nodes = chain.nodes.clone();
    let registered_nodes = RegisteredNodes { nodes };

    HttpResponse::Ok().json(registered_nodes)
}

async fn synchronize_chain<C: Consensus>(
    chain_data: &Arc<Mutex<Chain<C>>>,
    chain_info: &Arc<Mutex<ChainInfo>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let nodes = {
        let chain = chain_data.lock().await;
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
                    consensus: chain_data.lock().await.consensus.clone(),
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
        let mut chain = chain_data.lock().await;
        if new_chain.len() > chain.chain.len() {
            chain.chain = new_chain;

            let last_block = chain.chain.last().unwrap();
            let mut info = chain_info.lock().await;
            info.length = chain.chain.len() as u64;
            info.last_hash = last_block.hash.clone();

            println!("Chain updated. New length {}", max_len);
        }
    }
    Ok(())
}

pub async fn submit_message<C: Consensus>(
    message_queue: web::Data<MessageQueue>,
    app_state: web::Data<AppState<C>>,
    req: web::Json<MessageRequest>,
) -> impl Responder {
    match message_queue.submit_message(req.message.clone()).await {
        Ok(_) => {
            let _ = app_state.mining_tx.send(MiningCommand::StartMining).await;
            HttpResponse::Ok().body("Message queued successfully")
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

pub async fn generate_block<C: Consensus>(
    data: web::Data<Arc<Mutex<Chain<C>>>>,
    app_state: web::Data<AppState<C>>,
) -> impl Responder
where
    C::Proof: Serialize,
    Block<C::Proof>: Serialize,
{
    let (block_option, nodes, chain_len) = {
        let mut chain = data.lock().await;
        let timestamp = chrono::Utc::now().timestamp();
        let block = chain.new_block(timestamp).await;
        let nodes = if block.is_some() {
            chain.nodes.clone()
        } else {
            HashSet::new()
        };
        let chain_len = chain.chain.len() as u64;
        (block, nodes, chain_len)
    };

    match block_option {
        Some(block) => {
            let block_clone = block.clone();
            let mut info = app_state.chain_info.lock().await;
            info.length = chain_len;
            info.last_hash = block.hash.clone();
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
    data: web::Data<Arc<Mutex<Chain<C>>>>,
) -> impl Responder {
    let chain = data.lock().await;

    let pending_count = chain.mempool.pending_count();

    #[derive(Serialize)]
    struct MempoolStatus {
        pending_transactions: usize,
    }

    HttpResponse::Ok().json(MempoolStatus {
        pending_transactions: pending_count,
    })
}

pub async fn start_mining<C: Consensus>(app_state: web::Data<AppState<C>>) -> impl Responder {
    let _ = app_state.mining_tx.send(MiningCommand::StartMining).await;
    HttpResponse::Ok().body("Mining Started")
}

pub async fn stop_mining<C: Consensus>(app_state: web::Data<AppState<C>>) -> impl Responder {
    let _ = app_state.mining_tx.send(MiningCommand::StopMining).await;
    HttpResponse::Ok().body("Stopped Mining")
}

fn configure_api_routes<C: Consensus>(cfg: &mut web::ServiceConfig) {
    cfg.route("/chain", web::get().to(get_chain::<C>))
        .route("/block", web::post().to(post_block::<C>))
        .route("/generate", web::post().to(generate_block::<C>))
        .route("/submit", web::post().to(submit_message::<C>))
        .route("/pending", web::get().to(get_pending_transactions::<C>))
        .route("/nodes", web::get().to(get_nodes::<C>))
        .route("/nodes/register", web::post().to(register_node::<C>))
        .route("/mining/start", web::post().to(start_mining::<C>))
        .route("/mining/end", web::post().to(stop_mining::<C>))
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
    let chain_data = Arc::new(tokio::sync::Mutex::new(chain));
    let web_chain_data = web::Data::new(chain_data.clone());
    println!("Starting rustchain node on port {}", address);

    let (block_tx, mut block_rx) = tokio::sync::mpsc::channel::<(Block<C::Proof>, Vec<String>)>(32);

    let mempool = {
        let chain = chain_data.lock().await;
        Arc::new(Mutex::new(chain.mempool.clone()))
    };

    let message_queue = MessageQueue::new(mempool.clone());
    let message_queue_data = web::Data::new(message_queue.clone());

    let chain_info = {
        let chain = chain_data.lock().await;
        let last_block = chain.chain.last().unwrap();
        Arc::new(Mutex::new(ChainInfo {
            length: chain.chain.len() as u64,
            last_hash: last_block.hash.clone(),
        }))
    };

    let mining_interface = MiningInterface {
        mempool_accessor: mempool.clone(),
        chain_info: chain_info.clone(),
        consensus: chain_data.lock().await.consensus.clone(),
        block_channel: block_tx,
    };

    let (mut mining_coordinator, mining_tx) = MiningCoordinator::new(mining_interface, 100);

    let block_receiver_chain_data = chain_data.clone();
    let block_receiver_chain_info = chain_info.clone();

    tokio::spawn(async move {
        while let Some((block, message_ids)) = block_rx.recv().await {
            let mut chain = block_receiver_chain_data.lock().await;
            println!(
                "Received Block #{}, removing {} messages",
                block.index,
                message_ids.len()
            );
            //TODO: CRITICAL: Only remove if the msgs are in the chain.
            chain.mempool.remove_messages(&message_ids);
            if chain.chain.len() as u64 == block.index {
                chain.chain.push(block.clone());

                let block_hash = block.hash.clone();
                let block_index = block.index;
                let nodes = chain.nodes.clone();
                {
                    let mut info = block_receiver_chain_info.lock().await;
                    info.length = chain.chain.len() as u64;
                    info.last_hash = block_hash;
                }

                let block_clone = block.clone();
                tokio::spawn(async move {
                    if let Err(e) = client::broadcast_block::<C>(&nodes, &block_clone, None).await {
                        eprintln!("Error broadcasting mined block: {}", e);
                    }
                });
                println!("Block #{} added to chain", block_index);
            } else {
                println!(
                    "Chain changed during mining, discarding block #{}",
                    block.index
                );
                if let Err(e) =
                    synchronize_chain(&block_receiver_chain_data, &block_receiver_chain_info).await
                {
                    eprintln!("Error synching chain after discard", e);
                }
            }
        }
    });

    let mining_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2) // TODO: parametrize
        .enable_all()
        .build()
        .unwrap();

    let mining_handle = mining_runtime.spawn(async move {
        mining_coordinator.run().await;
    });

    let mining_runtime = Arc::new(mining_runtime);

    let app_state = web::Data::new(AppState::<C> {
        poll_interval_s: super::POLL_INTERVAL_S,
        chain_file: chain_file.clone(),
        mining_tx: mining_tx.clone(),
        mining_handle: Some(mining_handle),
        chain_info: chain_info.clone(),
        _consensus_type: std::marker::PhantomData,
    });

    let _ = mining_tx.send(MiningCommand::StartMining).await;

    let sync_chain_data = chain_data.clone();
    let sync_chain_info = chain_info.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = synchronize_chain(&sync_chain_data, &sync_chain_info).await {
                eprintln!("Error synchronizing chain: {}", e);
            }
        }
    });

    let persistence_data = chain_data.clone();
    let chain_file_clone = chain_file.clone();
    tokio::spawn(async move {
        // TODO: Parametrize
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let save_result = {
                let chain = persistence_data.lock().await;
                chain.save_to_file(&chain_file_clone)
            };
            if let Err(e) = save_result {
                eprintln!("Error saving chain: {}", e);
            } else {
                println!("Chain saved to {}", chain_file_clone);
            }
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web_chain_data.clone())
            .app_data(app_state.clone())
            .app_data(message_queue_data.clone())
            .configure(configure_api_routes::<C>)
            .configure(configure_frontend_routes::<C>)
            .app_data(web::Data::new(mining_runtime.clone()))
    })
    .bind(address)?
    .run()
    .await
}
