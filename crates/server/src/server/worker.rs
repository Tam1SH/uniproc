use crate::server::main_loop::handle_connection;
use crate::server::message_protocol::MessageProtocol;
use crate::server::runtime;
use crate::server::transport::raw::{TransportAcceptor, TransportBuilder};
use crate::server::transport::stream::AcceptorBuilder;
use crate::server::ServiceHandler;
use anyhow::{Context, Result};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

pub struct ServerWorker<P: MessageProtocol, H: ServiceHandler<P>> {
    phantom: PhantomData<(P, H)>,
}

impl<P: MessageProtocol, H: ServiceHandler<P>> ServerWorker<P, H> {
    pub fn spawn<B: TransportBuilder>(
        core_id: usize,
        builder: B,
        handler: H,
    ) -> anyhow::Result<()> {
        runtime::spawn_on(core_id, move || async move {
            let acceptor = match builder.bind().await {
                Ok(a) => a,
                Err(e) => {
                    error!(core_id, error = %e, "Failed to bind listener");
                    return;
                }
            };

            info!(core_id, "Server worker listening");

            loop {
                match acceptor.accept().await {
                    Ok(transport) => {
                        let h = handler.clone();
                        compio::runtime::spawn(async move {
                            if let Err(e) = handle_connection::<P, H, _>(transport, h).await {
                                error!(error = %e, "Session error");
                            }
                        })
                        .detach();
                    }
                    Err(e) => {
                        error!(core_id, error = %e, "Accept error");
                        compio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            }
        });
        Ok(())
    }
}
