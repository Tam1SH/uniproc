use crate::server::client_per_core::ClientPerCore;
use crate::server::message_protocol::{MessageProtocol, ResponseGuard};
use crate::server::runtime;
use crate::server::transport::raw::TransportConnector;
use crate::server::transport::stream::Connector;
use anyhow::{anyhow, Result};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

#[derive(Clone, Copy, Debug)]
pub enum Priority {
    Critical,
    Normal,
    Bulk,
}

struct CallRequest<P: MessageProtocol> {
    req: P::Request,
    resp_tx: oneshot::Sender<Result<ResponseGuard<P>>>,
}

pub struct Client<P: MessageProtocol> {
    critical_lane: flume::Sender<CallRequest<P>>,
    normal_lanes: Rc<Vec<flume::Sender<CallRequest<P>>>>,
    rr_normal: Rc<AtomicUsize>,
    bulk_lanes: Rc<Vec<flume::Sender<CallRequest<P>>>>,
    rr_bulk: Rc<AtomicUsize>,
}

impl<P: MessageProtocol> Clone for Client<P> {
    fn clone(&self) -> Self {
        Self {
            bulk_lanes: self.bulk_lanes.clone(),
            critical_lane: self.critical_lane.clone(),
            normal_lanes: self.normal_lanes.clone(),
            rr_bulk: self.rr_bulk.clone(),
            rr_normal: self.rr_normal.clone(),
        }
    }
}

impl<P: MessageProtocol + 'static> Client<P> {
    pub async fn connect_with<F, C>(
        limit_cores: usize,
        connector_factory: F,
    ) -> anyhow::Result<Self>
    where
        F: Fn(usize) -> C,
        C: TransportConnector,
    {
        let available = runtime::core_count();
        let num_cores = limit_cores.min(available);

        let connectors: Vec<C> = (0..num_cores).map(connector_factory).collect();

        Self::connect(connectors, num_cores).await
    }

    pub async fn connect<C: TransportConnector>(
        connectors: Vec<C>,
        limit_cores: usize,
    ) -> Result<Self> {
        let available_runtime_cores = runtime::core_count();

        let num_cores = limit_cores.min(available_runtime_cores);

        assert!(num_cores >= 1, "At least 1 core required for client");

        let mut all_worker_txs = Vec::with_capacity(num_cores);
        let (init_tx, init_rx) = flume::bounded::<Result<usize>>(num_cores);

        for (core_id, connector) in connectors.into_iter().enumerate() {
            let (worker_tx, worker_rx) = flume::unbounded::<CallRequest<P>>();
            all_worker_txs.push(worker_tx);

            let sync_tx = init_tx.clone();

            runtime::spawn_on(core_id, move || async move {
                let connect_res = async {
                    let transport = connector.connect().await?;
                    ClientPerCore::<P>::connect(transport).await
                }
                .await;

                match connect_res {
                    Ok(mut client) => {
                        debug!(core_id, "Worker connected successfully");
                        let _ = sync_tx.send_async(Ok(core_id)).await;

                        while let Ok(msg) = worker_rx.recv_async().await {
                            let res = client.call(msg.req).await;
                            if msg.resp_tx.send(res).is_err() {
                                warn!(
                                    core_id,
                                    "Caller dropped response channel before receiving result"
                                );
                            }
                        }
                        info!(core_id, "Worker shutting down (channel closed)");
                    }
                    Err(e) => {
                        error!(core_id, error = %e, "Worker failed to connect");
                        let _ = sync_tx.send_async(Err(e)).await;
                    }
                }
            });
        }

        for _ in 0..num_cores {
            match init_rx.recv_async().await {
                Ok(Ok(core_id)) => {
                    debug!(core_id, "Worker sync successfully");
                }
                Ok(Err(e)) => {
                    error!(error = %e, "Failed to initialize one or more core workers");
                    return Err(e);
                }
                Err(_) => {
                    return Err(anyhow!("Init channel closed prematurely"));
                }
            }
        }

        let critical_lane = all_worker_txs[0].clone();
        let remaining_workers = &all_worker_txs[1..];
        let count = remaining_workers.len();

        let (normal_workers, bulk_workers) = if count == 1 {
            (
                vec![remaining_workers[0].clone()],
                vec![remaining_workers[0].clone()],
            )
        } else {
            let mid = (count + 1) / 2;
            (
                remaining_workers[0..mid].to_vec(),
                remaining_workers[mid..].to_vec(),
            )
        };

        info!(
            critical = 1,
            normal = normal_workers.len(),
            bulk = bulk_workers.len(),
            "FatClient pool distribution complete"
        );

        Ok(Self {
            critical_lane,
            normal_lanes: Rc::new(normal_workers),
            rr_normal: Rc::new(AtomicUsize::new(0)),
            bulk_lanes: Rc::new(bulk_workers),
            rr_bulk: Rc::new(AtomicUsize::new(0)),
        })
    }

    pub async fn call(&self, req: P::Request, prio: Priority) -> Result<ResponseGuard<P>> {
        let tx = match prio {
            Priority::Critical => &self.critical_lane,
            Priority::Normal => {
                let idx = self.rr_normal.fetch_add(1, Ordering::Relaxed) % self.normal_lanes.len();
                &self.normal_lanes[idx]
            }
            Priority::Bulk => {
                let idx = self.rr_bulk.fetch_add(1, Ordering::Relaxed) % self.bulk_lanes.len();
                &self.bulk_lanes[idx]
            }
        };

        let (resp_tx, resp_rx) = oneshot::channel();

        if let Err(_) = tx.send_async(CallRequest { req, resp_tx }).await {
            error!(?prio, "Failed to send request: worker task died");
            return Err(anyhow!("Worker task dropped"));
        }

        resp_rx.await.map_err(|e| {
            error!(?prio, error = %e, "Worker failed to provide response (oneshot closed)");
            anyhow!("Worker response cancelled")
        })?
    }
}
