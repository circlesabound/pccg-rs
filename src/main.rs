#[macro_use]
extern crate log;

mod engine;
mod models;
mod server;

use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    logging_init();

    info!("Reading server config");
    let args: Vec<String> = std::env::args().collect();
    let server_config = server::ServerConfig::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {:?}", err);
        std::process::exit(1);
    });

    info!("Initialising db");
    let db = models::Db::new();

    info!("Initialising engine api");
    let api = Arc::new(Mutex::new(engine::Api::new(db)));

    info!("Starting web server");
    let routes = server::build_routes(api);
    warp::serve(routes)
        .run(server_config.get_socket_addr())
        .await;
}

fn logging_init() {
    // If log level is not explicitly set,
    // set to info by default
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
}
