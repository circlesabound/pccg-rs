use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[derive(Debug)]
pub struct ServerConfig {
    pub port: u16,
}

impl ServerConfig {
    pub fn new(args: &[String]) -> Result<ServerConfig, ArgumentError> {
        if let Some(arg) = args.get(1) {
            if let Ok(port) = arg.parse() {
                let config = ServerConfig { port };
                trace!("Read ServerConfig as {:?}", config);
                return Ok(config);
            } else {
                return Err(ArgumentError("Port is not a valid number"));
            }
        }

        Err(ArgumentError("Missing argument for port"))
    }

    pub fn get_socket_addr(&self) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), self.port))
    }
}

#[derive(Debug)]
pub struct ArgumentError<'a>(&'a str);
