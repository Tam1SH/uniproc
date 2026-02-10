use crate::server::transport::raw::peer::PeerConfig;
use crate::server::transport::raw::stream::{GenericStreamBuilder, GenericStreamConnector};
use crate::server::transport::stream::tcp::{TcpAcceptorBuilder, TcpConnector};

#[derive(Clone)]
pub struct TcpTransport {
    host: String,
    base_port: u32,
    config: PeerConfig,
}

impl TcpTransport {
    pub fn new(host: impl Into<String>, base_port: u32, config: PeerConfig) -> Self {
        Self {
            host: host.into(),
            base_port,
            config,
        }
    }

    pub fn server_builder(&self, core_id: usize) -> GenericStreamBuilder<TcpAcceptorBuilder> {
        let addr = format!("{}:{}", self.host, self.base_port + core_id as u32);
        GenericStreamBuilder::new(
            TcpAcceptorBuilder::new(addr.parse().unwrap()),
            self.config.clone(),
        )
    }

    pub fn client_connector(&self, core_id: usize) -> GenericStreamConnector<TcpConnector> {
        let addr = format!("{}:{}", self.host, self.base_port + core_id as u32);
        GenericStreamConnector::new(
            TcpConnector::new(addr.parse().unwrap()),
            self.config.clone(),
        )
    }
}
