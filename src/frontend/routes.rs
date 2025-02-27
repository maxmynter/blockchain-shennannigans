use crate::blockchain::{Block, Chain, Consensus};
use actix_web::{web, HttpResponse, Responder};
use askama::Template;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Template)]
#[template(path = "blockchain.html")]
struct BlockchainTemplate<'a, P> {
    blocks: &'a Vec<Block<P>>,
}

#[derive(Template)]
#[template(path = "block.html")]
struct BlockTemplate<'a, P> {
    block: &'a Block<P>,
}

pub async fn render_blockchain<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();

    let template = BlockchainTemplate {
        blocks: &chain.chain,
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
