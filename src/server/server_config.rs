pub struct ServerConfig {
    pub port: u16
}

impl ServerConfig {
    pub fn new(args: &[String]) -> Result<ServerConfig, ArgumentError> {
        if let Some(arg) = args.get(1) {
            if let Ok(port) = arg.parse() {
                return Ok(ServerConfig { port })
            } else {
                return Err(ArgumentError("Port is not a valid number"))
            }
        }

        Err(ArgumentError("Missing argument for port"))
    }

    pub fn get_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::new(0, 0, 0, 0),
            self.port
        ))
    }
}

#[derive(Debug)]
pub struct ArgumentError<'a>(&'a str);
