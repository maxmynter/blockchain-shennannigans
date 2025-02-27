mod api;
mod blockchain;
mod frontend;
mod utils;

use actix_web::{web, App, HttpServer};
use api::server::{generate_block, get_chain, get_nodes, post_block, register_node};
use blockchain::{Chain, ProofOfWork};
use frontend::routes::{register_node_form, render_blockchain, render_nodes, submit_message};
use std::sync::Mutex;

fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/chain", web::get().to(get_chain::<ProofOfWork>))
        .route("/block", web::post().to(post_block::<ProofOfWork>))
        .route("/generate", web::post().to(generate_block::<ProofOfWork>))
        .route("/nodes", web::get().to(get_nodes::<ProofOfWork>))
        .route(
            "/nodes/register",
            web::post().to(register_node::<ProofOfWork>),
        );
}

fn configure_frontend_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(render_blockchain::<ProofOfWork>))
        .route("/message", web::post().to(submit_message::<ProofOfWork>))
        .route("/web/nodes", web::get().to(render_nodes::<ProofOfWork>))
        .route(
            "/web/nodes/register",
            web::post().to(register_node_form::<ProofOfWork>),
        );
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    if let Some(port) = std::env::args().nth(1) {
        let port = port.parse::<u16>().expect("Invalid Port Number");
        let mut chain = Chain::new(ProofOfWork::new(4));
        if port != 8080 {
            chain.register_node(&format!("http://127.0.0.1:8080"));
        }
        if port != 8081 {
            chain.register_node(&format!("http://127.0.0.1:8081"));
        }
        println!("Starting node on port {}", port);

        let chain_data = web::Data::new(Mutex::new(chain));

        HttpServer::new(move || {
            App::new()
                .app_data(chain_data.clone())
                .configure(configure_api_routes)
                .configure(configure_frontend_routes)
        })
        .bind(format!("127.0.0.1:{}", port))?
        .run()
        .await
    } else {
        println!("Please provide port number");
        Ok(())
    }
}
