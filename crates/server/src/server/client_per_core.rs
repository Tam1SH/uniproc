use crate::align_buffer::AlignedBuffer;
use crate::server::builder::NoOpHandler;
use crate::server::main_loop::session_loop;
use crate::server::message_protocol::{Message, MessageProtocol, ResponseGuard};
use crate::server::tpc_pool::TpcPool;
use crate::server::transport::raw::{RawTransport, SendHandle};
use anyhow::anyhow;
use dashmap::DashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub struct ClientPerCore<P: MessageProtocol> {
    peer: SendHandle,
    pending: Rc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    next_id: Rc<AtomicU64>,
    phantom: PhantomData<P>,
}

impl<P: MessageProtocol> Clone for ClientPerCore<P> {
    fn clone(&self) -> Self {
        Self {
            next_id: self.next_id.clone(),
            phantom: PhantomData,
            peer: self.peer.clone(),
            pending: self.pending.clone(),
        }
    }
}

impl<P: MessageProtocol> ClientPerCore<P> {
    pub fn new(
        peer: SendHandle,
        pending: Rc<DashMap<u64, oneshot::Sender<AlignedBuffer>>>,
    ) -> Self {
        Self {
            peer,
            pending,
            next_id: Rc::new(AtomicU64::new(0)),
            phantom: PhantomData,
        }
    }

    pub async fn connect<T: RawTransport>(transport: T) -> anyhow::Result<Self> {
        let (peer, rx) = transport.decompose()?;
        let pending = Rc::new(DashMap::new());

        let h_clone = peer.clone();
        let p_clone = pending.clone();
        compio::runtime::spawn(async move {
            session_loop::<P, _>(rx, h_clone, p_clone, NoOpHandler).await;
        })
        .detach();

        Ok(Self {
            peer,
            pending,
            next_id: Rc::new(AtomicU64::new(0)),
            phantom: PhantomData,
        })
    }

    pub async fn call(&mut self, req: P::Request) -> anyhow::Result<ResponseGuard<P>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        self.pending.insert(id, tx);

        let buf = TpcPool::acquire_body(0);
        let msg = Message::Request { id, payload: req };
        let buf = P::encode(msg, buf)?;

        self.peer.send(buf).await?;

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
