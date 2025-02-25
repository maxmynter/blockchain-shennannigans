use crate::blockchain::{Block, Chain, Consensus};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct ChainWrapper<P: Serialize + DeserializeOwned> {
    chain: Vec<Block<P>>,
}

// Get /chain: Returns current chain
async fn get_chain<C: Consensus>(data: web::Data<Chain<C>>) -> impl Responder {
    let wrapper = ChainWrapper {
        chain: data.chain.clone(),
    };
    HttpResponse::Ok().json(wrapper)
}

// Post /block : Receives a new block and validates it
async fn post_block<C: Consensus>(
    data: web::Data<Chain<C>>,
    block: web::Json<Block<C: Proof>>,
) -> impl Responder {
    let mut chain = data.into_inner();
    if chain.consesnsus = validate(&chain, &block) {
        chain.chain.push(block.into_inner());
        HttpResponse::Ok().body("Block added")
    } else {
        HttpResponse::BadRequest().body("Invalid Block")
    }
}

// Start server with given chain and address
pub async fn run_server<C: Consensus>(chain: Chain<C>, address: &str) -> std::io::Result<()> {
    let chain_data = web::Data::new(chain);
    HttpServer::new(move || {
        App::new()
            .app_data(chain_data.clone())
            .route("/chain", web::get().to(get_chain::<C>))
            .route("/block", web::post().to(post_block::<C>))
    })
    .bind(address)?
    .run()
    .await
}
