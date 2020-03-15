#[macro_use]
extern crate log;

mod engine;
mod models;
mod server;
mod storage;

use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() {
    logging_init();
    let ctrlc_future = ctrlc_handler_init();

    info!("Reading server config");
    let args: Vec<String> = std::env::args().collect();
    let server_config = server::ServerConfig::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {:?}", err);
        std::process::exit(1);
    });

    info!("Initialising db");
    let db = storage::Db::new();

    info!("Initialising engine api");
    let api = Arc::new(engine::Api::new(db.cards().into_iter()));

    info!("Starting web server");
    let routes = server::build_routes(api);
    let (_, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(
            server_config.get_socket_addr(),
            ctrlc_future);
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
fn ctrlc_handler_init() -> impl futures::Future<Output = ()> {
    async {
        signal::ctrl_c().await.ok();
        info!("SIGINT detected");
    }
}
