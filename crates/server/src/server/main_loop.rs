use crate::align_buffer::AlignedBuffer;
use crate::server::message_protocol::{Message, MessageProtocol};
use crate::server::tpc_pool::TpcPool;
use crate::server::transport::raw::{IncomingMsg, MsgBatch, RawTransport, SendHandle};
use crate::server::ServiceHandler;
use compio::buf::IoBuf;
use std::collections::HashMap;
use std::rc::Rc;

use dashmap::DashMap;
use std::sync::Arc;
use tracing::error;
pub async fn session_loop<P, H>(
    incoming_rx: flume::Receiver<AlignedBuffer>,
    peer: SendHandle,
    pending: Rc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    handler: H,
) where
    P: MessageProtocol,
    H: ServiceHandler<P>,
{
    while let Ok(first_raw) = incoming_rx.recv_async().await {
        let mut in_batch = MsgBatch::new();
        in_batch.push(first_raw);

        while in_batch.len() < in_batch.capacity() {
            if let Ok(next_raw) = incoming_rx.try_recv() {
                in_batch.push(next_raw);
            } else {
                break;
            }
        }

        let handler = handler.clone();
        let mut pending = pending.clone();
        let peer = peer.clone();
        compio::runtime::spawn(async move {
            let mut out_batch = MsgBatch::new();

            for raw in in_batch {
                match P::decode(&raw.0) {
                    Ok(Message::Request { id, payload }) => {
                        if let Ok(resp) = handler.on_request(payload).await {
                            let out_buf = TpcPool::acquire_body(0);
                            if let Ok(final_buf) =
                                P::encode(Message::Response { id, payload: resp }, out_buf)
                            {
                                out_batch.push(final_buf);
                            }
                        }
                        TpcPool::release_body(raw);
                    }
                    Ok(Message::Push { payload }) => {
                        let _ = handler.on_request(payload).await;
                        TpcPool::release_body(raw);
                    }
                    Ok(Message::Response { id, .. }) => {
                        if let Some((_, tx)) = pending.remove(&id) {
                            let _ = tx.send(raw);
                        } else {
                            TpcPool::release_body(raw);
                        }
                    }
                    Err(e) => {
                        error!("Protocol decode error: {e}");
                        TpcPool::release_body(raw);
                    }
                }
            }

            if !out_batch.is_empty() {
                let _ = peer.send_batch(out_batch).await;
            }
        })
        .detach();
    }
}

pub async fn handle_connection<P, H, T>(transport: T, handler: H) -> anyhow::Result<()>
where
    P: MessageProtocol,
    T: RawTransport,
    H: ServiceHandler<P>,
{
    let (peer_handle, rx) = transport.decompose()?;

    let pending = Rc::new(DashMap::new());

    session_loop::<P, H>(rx, peer_handle, pending, handler).await;

    Ok(())
}
