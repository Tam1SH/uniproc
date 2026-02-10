use crate::server::commands::{Request, Response};
use crate::server::main_loop::handle_connection;
use crate::server::message_protocol::MessageProtocol;
use crate::server::transport::raw::stream::{GenericStreamAcceptor, StreamTransport};
use crate::server::transport::raw::TransportBuilder;
use crate::server::worker::ServerWorker;
use crate::server::{runtime, ServiceHandler};
use anyhow::{anyhow, Context};
use compio::net::{TcpListener, TcpStream};
use std::marker::PhantomData;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct NoOpHandler;
impl<P: MessageProtocol> ServiceHandler<P> for NoOpHandler {
    async fn on_request(
        &self,
        _: &<P as MessageProtocol>::RequestView,
    ) -> anyhow::Result<<P as MessageProtocol>::Response> {
        Err(anyhow!("no op"))
    }
}

pub type BuilderFactory<B> = Box<dyn Fn(usize) -> B + Send + Sync>;

pub struct RpcBuilder<P: MessageProtocol, H: ServiceHandler<P>, B: TransportBuilder> {
    cores: usize,
    handler: H,
    builder_factory: Option<BuilderFactory<B>>,
    phantom: PhantomData<P>,
}

pub fn setup<P: MessageProtocol>() -> RpcBuilder<P, NoOpHandler, EmptyBuilder> {
    RpcBuilder {
        cores: num_cpus::get(),
        handler: NoOpHandler,
        builder_factory: None,
        phantom: PhantomData,
    }
}

pub struct EmptyBuilder;
impl TransportBuilder for EmptyBuilder {
    type Transport = StreamTransport<TcpStream>;
    type Acceptor = GenericStreamAcceptor<TcpListener>;
    async fn bind(self) -> std::io::Result<Self::Acceptor> {
        unreachable!()
    }
}

impl<P: MessageProtocol, H: ServiceHandler<P>, B: TransportBuilder> RpcBuilder<P, H, B> {
    pub fn cores(mut self, cores: usize) -> Self {
        self.cores = cores;
        self
    }

    pub fn service<NewH: ServiceHandler<P>>(self, handler: NewH) -> RpcBuilder<P, NewH, B> {
        RpcBuilder {
            cores: self.cores,
            handler,
            builder_factory: self.builder_factory,
            phantom: PhantomData,
        }
    }

    pub fn with_transport<NewB: TransportBuilder>(
        self,
        factory: impl Fn(usize) -> NewB + Send + Sync + 'static,
    ) -> RpcBuilder<P, H, NewB> {
        RpcBuilder {
            cores: self.cores,
            handler: self.handler,
            builder_factory: Some(Box::new(factory)),
            phantom: PhantomData,
        }
    }

    pub async fn run(self) -> anyhow::Result<std::convert::Infallible> {
        let handler = self.handler;
        let factory = self
            .builder_factory
            .ok_or_else(|| anyhow!("Transport builder not set. Call .with_transport()"))?;

        let available_cores = runtime::core_count();
        if available_cores == 0 {
            return Err(anyhow!("Runtime pool is not initialized."));
        }

        let cores_to_use = self.cores.min(available_cores);

        info!(cores = cores_to_use, "Starting RPC server workers");

        for core_id in 0..cores_to_use {
            let h = handler.clone();
            let builder = factory(core_id);

            ServerWorker::<P, H>::spawn(core_id, builder, h)
                .with_context(|| format!("Failed to spawn worker on core {}", core_id))?;
        }

        loop {
            compio::time::sleep(Duration::from_secs(3600)).await;
        }
    }
}
