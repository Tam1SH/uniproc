use crate::align_buffer::AlignedBuffer;
use crate::server::transport::raw::peer::{Peer, PeerConfig};
use crate::server::transport::raw::{
    RawTransport, SendHandle, TransportAcceptor, TransportBuilder, TransportConnector,
};
use crate::server::transport::stream::{Acceptor, AcceptorBuilder, AsyncStream, Connector};
use std::io;

pub struct StreamTransport<S: AsyncStream> {
    pub stream: S,
    pub config: PeerConfig,
}

impl<S: AsyncStream> RawTransport for StreamTransport<S> {
    fn decompose(self) -> anyhow::Result<(SendHandle, flume::Receiver<AlignedBuffer>)> {
        Peer::new(self.stream, self.config).map_err(Into::into)
    }
}

pub struct GenericStreamAcceptor<A: Acceptor> {
    inner: A,
    config: PeerConfig,
}

impl<A: Acceptor> TransportAcceptor for GenericStreamAcceptor<A> {
    type Transport = StreamTransport<A::Stream>;

    async fn accept(&self) -> io::Result<Self::Transport> {
        let (stream, _) = self.inner.accept().await?;
        let transport = StreamTransport {
            stream,
            config: self.config.clone(),
        };
        Ok(transport)
    }
}

pub struct GenericStreamBuilder<B: AcceptorBuilder> {
    inner_builder: B,
    config: PeerConfig,
}

impl<B: AcceptorBuilder> GenericStreamBuilder<B> {
    pub fn new(inner_builder: B, config: PeerConfig) -> Self {
        Self {
            inner_builder,
            config,
        }
    }
}

impl<B: AcceptorBuilder> TransportBuilder for GenericStreamBuilder<B> {
    type Transport = StreamTransport<B::Stream>;
    type Acceptor = GenericStreamAcceptor<B::Acceptor>;

    async fn bind(self) -> io::Result<Self::Acceptor> {
        let acceptor = self.inner_builder.bind().await?;
        Ok(GenericStreamAcceptor {
            inner: acceptor,
            config: self.config,
        })
    }
}

pub struct GenericStreamConnector<C: Connector> {
    inner: C,
    config: PeerConfig,
}

impl<C: Connector> GenericStreamConnector<C> {
    pub fn new(inner: C, config: PeerConfig) -> Self {
        Self { inner, config }
    }
}

impl<C: Connector> TransportConnector for GenericStreamConnector<C> {
    type Transport = StreamTransport<C::Stream>;

    async fn connect(&self) -> anyhow::Result<Self::Transport> {
        let stream = self.inner.connect().await?;
        Ok(StreamTransport {
            stream,
            config: self.config.clone(),
        })
    }
}
