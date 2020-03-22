use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub compendium: CompendiumConfig,
    pub user_registry: UserRegistryConfig,
    pub server: ServerConfig,
}

#[derive(Clone, Deserialize)]
pub struct CompendiumConfig {
    pub directory: String,
}

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn get_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            self.ip.parse().unwrap(),
            self.port,
        ))
    }
}

#[derive(Clone, Deserialize)]
pub struct UserRegistryConfig {
    pub directory: String,
}
