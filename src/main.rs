#[macro_use]
extern crate log;

mod engine;
mod models;
mod server;
mod storage;

use crate::storage::fs::FsStore;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;
use tokio::{signal, task};

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

    let compendium_task_config = Arc::clone(&config);
    let compendium_task = task::spawn(async move {
        info!(
            "Loading compendium from {}",
            compendium_task_config.compendium.directory
        );
        let sw = time::Instant::now();
        let storage: FsStore<models::Card> =
            FsStore::new(PathBuf::from(&compendium_task_config.compendium.directory)).unwrap();
        let compendium = models::Compendium::from_storage(Arc::new(storage))
            .await
            .unwrap_or_else(|err| {
                panic!("Problem loading compendium: {:?}", err);
            });
        info!("Loaded compendium in {:?}", sw.elapsed());
        compendium
    });

    let user_registry_task_config = Arc::clone(&config);
    let user_registry_task = task::spawn(async move {
        info!(
            "Loading user registry from {}",
            user_registry_task_config.user_registry.directory
        );
        let sw = time::Instant::now();
        let storage: FsStore<models::User> = FsStore::new(PathBuf::from(
            &user_registry_task_config.user_registry.directory,
        ))
        .unwrap();
        let user_registry = models::UserRegistry::from_storage(Arc::new(storage))
            .await
            .unwrap_or_else(|err| {
                panic!("Problem loading user registry: {:?}", err);
            });
        info!("Loaded user registry in {:?}", sw.elapsed());
        user_registry
    });

    let compendium = compendium_task.await.unwrap();
    let user_registry = user_registry_task.await.unwrap();

    info!("Initialising engine api");
    let api = engine::Api::new(compendium, user_registry).await;
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
