use crate::api::{client, server};
use crate::blockchain::{Block, Chain, Consensus};
use actix_web::rt::spawn;
use actix_web::{web, HttpResponse, Responder};
use askama::Template;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Template)]
#[template(path = "views/dashboard.html")]
struct DashboardTemplate<'a, P: std::fmt::Display> {
    blocks: &'a Vec<Block<P>>,
    nodes: &'a HashSet<String>,
    poll_interval_s: u64,
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
#[template(path = "components/display_chain.html")]
struct BlocksListTemplate<'a, P: std::fmt::Display> {
    blocks: &'a Vec<Block<P>>,
}

#[derive(Template)]
#[template(path = "responses/all_nodes.html")]
struct NodesListTemplate {
    nodes: Vec<String>,
}

pub async fn render_dashboard<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    app_state: web::Data<server::AppState>,
) -> impl Responder {
    let chain = data.lock().unwrap();

    let template = DashboardTemplate {
        blocks: &chain.chain,
        nodes: &chain.nodes,
        poll_interval_s: app_state.poll_interval_s,
    };

    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn render_blocks_list<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let chain = data.lock().unwrap();
    let order = query.get("order").map(|s| s.as_str()).unwrap_or("desc");
    let mut blocks = chain.chain.clone();
    if order == "asc" {
        blocks.reverse();
    }

    let template = BlocksListTemplate { blocks: &blocks };
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
        chain.new_block(message, timestamp).await
    };

    let template = BlockTemplate { block: &block };

    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}
pub async fn render_nodes_list<C: Consensus>(data: web::Data<Mutex<Chain<C>>>) -> impl Responder {
    let chain = data.lock().unwrap();
    let nodes: Vec<String> = chain.nodes.clone().into_iter().collect();

    let template = NodesListTemplate { nodes };
    match template.render() {
        Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn register_node_form<C: Consensus>(
    data: web::Data<Mutex<Chain<C>>>,
    form: web::Form<HashMap<String, String>>,
) -> impl Responder {
    let address = form.get("address").cloned().unwrap_or_default();

    if address.is_empty() {
        let template = NodeResultTemplate {
            success: false,
            message: "Node address cannot be empty".to_string(),
        };

        match template.render() {
            Ok(html) => HttpResponse::BadRequest()
                .content_type("text/html")
                .body(html),
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        }
    } else if !client::check_node_alive(&address).await {
        let template = NodeResultTemplate {
            success: false,
            message: format!("Node {} cannot be reached", address),
        };
        match template.render() {
            Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        }
    } else {
        {
            let mut chain = data.lock().unwrap();
            chain.add_node(&address);
        }
        let chain_clone = data.lock().unwrap().clone();

        let template = NodeResultTemplate {
            success: true,
            message: format!("Node {} registered successfully", address),
        };

        spawn(client::broadcast_node_registration(chain_clone, address));

        match template.render() {
            Ok(html) => HttpResponse::Ok().content_type("text/html").body(html),
            Err(err) => HttpResponse::InternalServerError()
                .content_type("text/html")
                .body(err.to_string()),
        }
    }
}
