use crate::server::commands::{Request, Response};
use crate::server::main_loop::handle_connection;
use crate::server::protocol::Protocol;
use crate::server::ServiceHandler;
use crate::vsock::{AsyncAcceptor, Listener, Splitable};
use anyhow::anyhow;
use compio::net::TcpListener;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

pub struct NoOpHandler;
impl<P: Protocol> ServiceHandler<P> for NoOpHandler {
    async fn on_request(
        &self,
        _: &<P as Protocol>::RequestView,
    ) -> anyhow::Result<<P as Protocol>::Response> {
        Err(anyhow!("no op"))
    }
}

pub fn setup<P: Protocol>() -> RpcBuilder<P, NoOpHandler> {
    RpcBuilder {
        port: 10000,
        handler: Arc::new(NoOpHandler),
        listener: None,
        phantom: PhantomData,
    }
}

pub struct RpcBuilder<P: Protocol, H: ServiceHandler<P>, L = Listener> {
    port: u32,
    handler: Arc<H>,
    listener: Option<L>,
    phantom: PhantomData<P>,
}

impl<P: Protocol, H: ServiceHandler<P>, L: AsyncAcceptor> RpcBuilder<P, H, L> {
    pub fn bind(mut self, port: u32) -> Self {
        self.port = port;
        self
    }

    pub fn service<NewH: ServiceHandler<P>>(self, handler: NewH) -> RpcBuilder<P, NewH, L> {
        RpcBuilder {
            port: self.port,
            handler: Arc::new(handler),
            listener: self.listener,
            phantom: PhantomData,
        }
    }

    pub fn with_listener<NewL: AsyncAcceptor>(self, listener: NewL) -> RpcBuilder<P, H, NewL> {
        RpcBuilder {
            port: self.port,
            handler: self.handler,
            listener: Some(listener),
            phantom: PhantomData,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let listener = self.listener.expect("listener not created");

        info!("Server listening on port {}", self.port);
        let handler = self.handler;

        compio::runtime::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok(stream) => {
                        let handler = handler.clone();
                        compio::runtime::spawn(async move {
                            if let Err(e) = handle_connection(stream, handler).await {
                                error!("Connection error: {:?}", e);
                            }
                        })
                        .detach();
                    }
                    Err(e) => {
                        error!("Accept error: {:?}", e);
                        compio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        })
        .detach();

        Ok(())
    }
}
