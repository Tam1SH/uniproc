use crate::server::transport::raw::peer::PeerConfig;
use crate::server::transport::raw::stream::{GenericStreamBuilder, GenericStreamConnector};
use crate::server::transport::stream::vsock::{VsockAcceptorBuilder, VsockConnector};

pub struct VsockTransport {
    cid: u32,
    base_port: u32,
    config: PeerConfig,
}

impl VsockTransport {
    pub fn new(cid: u32, base_port: u32, config: PeerConfig) -> Self {
        Self {
            cid,
            base_port,
            config,
        }
    }

    pub fn server_builder(&self, core_id: usize) -> GenericStreamBuilder<VsockAcceptorBuilder> {
        let port = self.base_port + core_id as u32;
        GenericStreamBuilder::new(
            VsockAcceptorBuilder::new(self.cid, port),
            self.config.clone(),
        )
    }

    pub fn client_connector(&self, core_id: usize) -> GenericStreamConnector<VsockConnector> {
        let port = self.base_port + core_id as u32;
        GenericStreamConnector::new(VsockConnector::new(self.cid, port), self.config.clone())
    }
}
