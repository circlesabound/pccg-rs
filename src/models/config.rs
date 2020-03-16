use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub compendium: CompendiumConfig,
}

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

#[derive(Clone, Deserialize)]
pub struct CompendiumConfig {
    pub file: String,
}

impl ServerConfig {
    pub fn get_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            self.ip.parse().unwrap(),
            self.port,
        ))
    }
}
