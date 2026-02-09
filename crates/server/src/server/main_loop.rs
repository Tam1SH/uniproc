use crate::align_buffer::AlignedBuffer;
use crate::peer::tpc_pool::TpcPool;
use crate::peer::{Peer, PeerHandle};
use crate::server::protocol::{Message, Protocol};
use crate::server::ServiceHandler;
use crate::vsock::AsyncStream;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::error;

pub async fn session_loop<P, H>(
    mut incoming_rx: flume::Receiver<AlignedBuffer>,
    peer_handle: PeerHandle,
    pending: Arc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    handler: Arc<H>,
) where
    P: Protocol,
    H: ServiceHandler<P>,
{
    while let Ok(raw) = incoming_rx.recv_async().await {
        let handler = handler.clone();
        let peer = peer_handle.clone();
        let pending = pending.clone();

        compio::runtime::spawn(async move {
            enum NextStep {
                ForwardResponse { id: u64 },
                DoNothing,
            }

            let step = match P::decode(&raw.0) {
                Ok(Message::Request { id, payload }) => {
                    if let Ok(resp) = handler.on_request(payload).await {
                        let out_buf = TpcPool::acquire_body(0);
                        if let Ok(final_buf) =
                            P::encode(Message::Response { id, payload: resp }, out_buf)
                        {
                            let _ = peer.send_raw(final_buf).await;
                        }
                    }
                    NextStep::DoNothing
                }
                Ok(Message::Response { id, .. }) => NextStep::ForwardResponse { id },
                Err(e) => {
                    error!("Protocol decode error: {e}");
                    NextStep::DoNothing
                }
            };

            match step {
                NextStep::ForwardResponse { id } => {
                    if let Some((_, tx)) = pending.remove(&id) {
                        let _ = tx.send(raw);
                    } else {
                        TpcPool::release_body(raw);
                    }
                }
                NextStep::DoNothing => {
                    TpcPool::release_body(raw);
                }
            }
        })
        .detach();
    }
}

pub async fn handle_connection<P, H, S>(stream: S, handler: Arc<H>) -> anyhow::Result<()>
where
    P: Protocol,
    S: AsyncStream + Clone + 'static,
    H: ServiceHandler<P>,
{
    let (peer_handle, rx) = Peer::new(stream)?;

    let pending = Arc::new(DashMap::new());

    session_loop::<P, H>(rx, peer_handle, pending, handler).await;

    Ok(())
}
