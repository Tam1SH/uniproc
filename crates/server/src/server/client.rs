use crate::align_buffer::AlignedBuffer;
use crate::peer::tpc_pool::TpcPool;
use crate::peer::{Peer, PeerHandle};
use crate::server::builder::NoOpHandler;
use crate::server::main_loop::session_loop;
use crate::server::protocol::{Message, Protocol, ResponseGuard};
use crate::vsock::AsyncStream;
use anyhow::anyhow;
use dashmap::DashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Client<P: Protocol> {
    peer: PeerHandle,
    pending: Arc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    next_id: Arc<AtomicU64>,
    phantom: PhantomData<P>,
}

impl<P: Protocol> Client<P> {
    pub fn new(
        peer: PeerHandle,
        pending: Arc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    ) -> Self {
        Self {
            peer,
            pending,
            next_id: Arc::new(AtomicU64::new(0)),
            phantom: PhantomData,
        }
    }

    pub async fn connect<S: AsyncStream + Clone + 'static>(stream: S) -> anyhow::Result<Self> {
        let (peer, rx) = Peer::new(stream)?;
        let pending = Arc::new(DashMap::new());

        let h_clone = peer.clone();
        let p_clone = pending.clone();
        compio::runtime::spawn(async move {
            session_loop::<P, _>(rx, h_clone, p_clone, Arc::new(NoOpHandler)).await;
        })
        .detach();

        Ok(Self {
            peer,
            pending,
            next_id: Arc::new(AtomicU64::new(0)),
            phantom: PhantomData,
        })
    }

    pub async fn call(&self, req: P::Request) -> anyhow::Result<ResponseGuard<P>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        self.pending.insert(id, tx);

        let buf = TpcPool::acquire_body(0);
        let msg = Message::Request { id, payload: req };
        let buf = P::encode(msg, buf)?;

        self.peer.send_raw(buf).await?;

        let response_buf = match compio::time::timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(b)) => b,
            Ok(Err(_)) => {
                self.pending.remove(&id);
                return Err(anyhow!("Channel closed"));
            }
            Err(_) => {
                self.pending.remove(&id);
                return Err(anyhow!("Request timeout"));
            }
        };

        P::decode(&response_buf.0).map_err(|e| anyhow!("Invalid response data: {e}"))?;

        Ok(ResponseGuard::new(response_buf))
    }
}
