use crate::blockchain::{Block, Chain, Consensus};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
struct ChainWrapper<P> {
    chain: Vec<Block<P>>,
}

#[derive(Deserialize)]
struct BlockRequest {
    data: String,
}

#[derive(Deserialize)]
struct NodeRequest {
    address: String,
}

// Get /chain: Returns current chain
async fn get_chain<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let wrapper = ChainWrapper {
        chain: chain.chain.clone(),
    };
    HttpResponse::Ok().json(wrapper)
}

// Post /block : Receives a new block and validates it
async fn post_block<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    block: web::Json<Block<C::Proof>>,
) -> impl Responder {
    let mut chain = data.lock().unwrap();
    if chain.consensus.validate(&chain, &block) {
        chain.chain.push(block.into_inner());
        HttpResponse::Ok().body("Block added")
    } else {
        HttpResponse::BadRequest().body("Invalid Block")
    }
}

async fn generate_block<C: Consensus>(
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

async fn register_node<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    req: web::Json<NodeRequest>,
) -> impl Responder {
    let mut chain = data.lock().unwrap();
    chain.register_node(&req.address);
    HttpResponse::Ok().body(format!("Node {} registered", req.address))
}

// Start server with given chain and address
pub async fn run_server<C: Consensus>(chain: Chain<C>, address: &str) -> std::io::Result<()>
where
    C::Proof: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let chain_data = web::Data::new(Mutex::new(chain));
    HttpServer::new(move || {
        App::new()
            .app_data(chain_data.clone())
            .route("/chain", web::get().to(get_chain::<C>))
            .route("/block", web::post().to(post_block::<C>))
            .route("/generate", web::post().to(generate_block::<C>))
            .route("/nodes/register", web::post().to(register_node::<C>))
    })
    .bind(address)?
    .run()
    .await
}
