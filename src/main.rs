#[macro_use]
extern crate log;

mod engine;
mod models;
mod server;
mod storage;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use storage::firestore::{Firestore, FirestoreClient};
use tokio::signal;

#[tokio::main]
async fn main() {
    logging_init();

    info!("Parsing config path from argv");
    let config_path = get_config_path_from_argv().unwrap_or_else(|err_msg| {
        eprintln!("Problem parsing arguments: {:?}", err_msg);
        std::process::exit(1);
    });

    info!(
        "Reading application config from {}",
        config_path.to_str().unwrap()
    );
    let config_str = fs::read_to_string(config_path).unwrap();
    let config: models::config::Config = toml::from_str(&config_str).unwrap();
    let config = Arc::new(config);

    let firestore = Arc::new(Firestore::new(&config.firestore.secret).await.unwrap());

    let users_firestore = FirestoreClient::new(Arc::clone(&firestore), None, "users".to_owned());
    let job_board = engine::job_board::JobBoard::new(FirestoreClient::new(
        Arc::clone(&firestore),
        None,
        "jobs".to_owned(),
    ))
    .await;
    let cards_firestore = FirestoreClient::new(Arc::clone(&firestore), None, "cards".to_owned());

    info!("Initialising engine api");
    let api = engine::Api::new(cards_firestore, job_board, users_firestore).await;
    let api = Arc::new(api);

    info!("Starting web server");
    let routes = server::build_routes(api);
    let (_, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(config.server.get_socket_addr(), ctrlc_handler());
    server.await;

    info!("Shutting down");
}

fn logging_init() {
    // If log level is not explicitly set,
    // set to info by default
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
}

/// Wrapper around tokio::signal::ctrl_c
async fn ctrlc_handler() {
    signal::ctrl_c().await.ok();
    info!("SIGINT detected");
}

fn get_config_path_from_argv() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().collect();
    args.get(1)
        .ok_or(String::from("Missing arg"))
        .map(|p| PathBuf::from(p))
}
