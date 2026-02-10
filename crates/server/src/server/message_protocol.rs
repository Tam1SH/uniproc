use crate::align_buffer::AlignedBuffer;
use crate::server::tpc_pool::TpcPool;
use anyhow::Result;

pub trait MessageProtocol: Send + Sync + 'static {
    type Request: Send + Sync + 'static;
    type Response: Send + Sync + 'static;

    type RequestView: ?Sized + Send + Sync;
    type ResponseView: ?Sized + Send + Sync;

    fn decode(data: &[u8]) -> Result<Message<&Self::RequestView, &Self::ResponseView>>;

    fn encode(
        msg: Message<Self::Request, Self::Response>,
        dest: AlignedBuffer,
    ) -> Result<AlignedBuffer>;
}

pub enum Message<Req, Res> {
    Request { id: u64, payload: Req },
    Response { id: u64, payload: Res },
    Push { payload: Req },
}

pub struct ResponseGuard<P: MessageProtocol> {
    buffer: AlignedBuffer,
    _phantom: std::marker::PhantomData<P>,
}

impl<P: MessageProtocol> ResponseGuard<P> {
    pub fn new(buffer: AlignedBuffer) -> Self {
        Self {
            buffer,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P: MessageProtocol> std::ops::Deref for ResponseGuard<P> {
    type Target = P::ResponseView;

    fn deref(&self) -> &Self::Target {
        let decoded = P::decode(&self.buffer.0).expect("Buffer was already validated");

        match decoded {
            Message::Response { payload, .. } => payload,
            _ => panic!("Not a response message"),
        }
    }
}

impl<P: MessageProtocol> Drop for ResponseGuard<P> {
    fn drop(&mut self) {
        TpcPool::release_body(std::mem::replace(
            &mut self.buffer,
            AlignedBuffer::default(),
        ));
    }
}
