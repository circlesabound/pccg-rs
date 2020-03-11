#[macro_use] extern crate log;

mod models;
mod server;

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
    let db = models::new_db();

    info!("Starting web server");
    warp::serve(server::get_routes(db))
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