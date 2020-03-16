#[macro_use]
extern crate log;

mod engine;
mod models;
mod server;

use std::sync::Arc;
use std::time;
use tokio::{signal, task};

#[tokio::main]
async fn main() {
    logging_init();

    info!("Reading server config");
    let args: Vec<String> = std::env::args().collect();
    let server_config = server::ServerConfig::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {:?}", err);
        std::process::exit(1);
    });

    let compendium_task = task::spawn(async {
        info!("Loading compendium");
        let sw = time::Instant::now();
        let compendium = models::Compendium::from_file("compendium.json")
            .await
            .unwrap_or_else(|err| {
                panic!("Problem loading compendium: {:?}", err);
            });
        info!("Loaded compendium in {:?}", sw.elapsed());
        compendium
    });

    let compendium = compendium_task.await.unwrap();

    info!("Initialising engine api");
    let api = engine::Api::new(compendium).await;
    let api = Arc::new(api);

    info!("Starting web server");
    let routes = server::build_routes(api);
    let (_, server) = warp::serve(routes)
        .bind_with_graceful_shutdown(server_config.get_socket_addr(), ctrlc_handler());
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
