mod server;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let server_config = server::ServerConfig::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {:?}", err);
        std::process::exit(1);
    });

    warp::serve(server::get_routes())
        .run(server_config.get_socket_addr())
        .await;
}
