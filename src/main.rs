mod api;
mod blockchain;
mod frontend;
mod utils;

use actix_web::{web, App, HttpServer};
use api::server::{generate_block, get_chain, post_block, register_node};
use blockchain::{Chain, ProofOfWork};
use frontend::routes::{render_blockchain, submit_message};
use std::sync::Mutex;

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
                // Api Routes
                .route("/chain", web::get().to(get_chain::<ProofOfWork>))
                .route("/block", web::post().to(post_block::<ProofOfWork>))
                .route("/generate", web::post().to(generate_block::<ProofOfWork>))
                .route(
                    "/nodes/register",
                    web::post().to(register_node::<ProofOfWork>),
                )
                // Frontend routes
                .route("/", web::get().to(render_blockchain::<ProofOfWork>))
                .route("/message", web::post().to(submit_message::<ProofOfWork>))
        })
        .bind(format!("127.0.0.1:{}", port))?
        .run()
        .await
    } else {
        println!("Please provide port number");
        Ok(())
    }
}
