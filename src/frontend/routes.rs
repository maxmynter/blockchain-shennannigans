use crate::blockchain::{Block, Chain, Consensus};
use actix_web::{web, HttpResponse, Responder};
use askama::Template;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Template)]
#[template(path = "views/blockchain.html")]
struct BlockchainTemplate<'a, P: std::fmt::Display> {
    blocks: &'a Vec<Block<P>>,
    nodes: &'a HashSet<String>,
}

#[derive(Template)]
#[template(path = "components/block.html")]
struct BlockTemplate<'a, P: std::fmt::Display> {
    block: &'a Block<P>,
}

#[derive(Template)]
#[template(path = "responses/node_result.html")]
struct NodeResultTemplate {
    success: bool,
    message: String,
}

#[derive(Template)]
#[template(path = "responses/sync_result.html")]
struct SyncResultTemplate {
    success: bool,
    message: String,
    blocks_added: usize,
}

#[derive(Template)]
#[template(path = "responses/all_nodes.html")]
struct AllNodesTemplate {
    nodes: Vec<String>,
}

pub async fn render_blockchain<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();

    let template = BlockchainTemplate {
        blocks: &chain.chain,
        nodes: &chain.nodes,
    };

    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn submit_message<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    form: web::Form<HashMap<String, String>>,
) -> impl Responder {
    let message = form.get("message").cloned().unwrap_or_default();
    let timestamp = chrono::Utc::now().timestamp();

    let block = {
        let mut chain = data.lock().unwrap();
        chain.new_block(message, timestamp)
    };

    let template = BlockTemplate { block: &block };

    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
pub async fn render_nodes<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let nodes: Vec<String> = chain.nodes.clone().into_iter().collect();

    let template = AllNodesTemplate { nodes };
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
