pub mod peer;
pub mod stream;

use crate::align_buffer::AlignedBuffer;
use smallvec::SmallVec;
use std::io;

pub type MsgBatch = SmallVec<[AlignedBuffer; 8]>;
pub type IncomingMsg = SmallVec<[AlignedBuffer; 8]>;

pub enum OutgoingMsg {
    Single(AlignedBuffer),
    Batch(MsgBatch),
}

#[derive(Clone)]
pub struct SendHandle {
    outgoing_tx: flume::Sender<OutgoingMsg>,
}

impl SendHandle {
    pub async fn send(&self, data: AlignedBuffer) -> anyhow::Result<()> {
        self.outgoing_tx
            .send_async(OutgoingMsg::Single(data))
            .await
            .map_err(|e| anyhow::anyhow!("transport dead: {}", e))
    }

    pub async fn send_batch(&self, msgs: MsgBatch) -> anyhow::Result<()> {
        self.outgoing_tx
            .send_async(OutgoingMsg::Batch(msgs))
            .await
            .map_err(|e| anyhow::anyhow!("transport dead: {}", e))
    }
}

pub trait RawTransport: 'static {
    fn decompose(self) -> anyhow::Result<(SendHandle, flume::Receiver<AlignedBuffer>)>;
}

pub trait TransportConnector: Send + Sync + 'static {
    type Transport: RawTransport;

    async fn connect(&self) -> anyhow::Result<Self::Transport>;
}

pub trait TransportAcceptor: 'static {
    type Transport: RawTransport;

    async fn accept(&self) -> io::Result<Self::Transport>;
}

pub trait TransportBuilder: Send + Sync + 'static {
    type Transport: RawTransport;
    type Acceptor: TransportAcceptor<Transport = Self::Transport>;

    async fn bind(self) -> io::Result<Self::Acceptor>;
}
