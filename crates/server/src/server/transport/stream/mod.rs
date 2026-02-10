use compio::io::{AsyncRead, AsyncReadAt, AsyncWrite};
use socket2::SockAddr;
use std::io;

pub mod adapters;
pub mod tcp;
pub mod vsock;

pub trait AsyncStream: Splitable + AsyncRead + AsyncWrite + Unpin + Clone + 'static {}

impl<T: Splitable + AsyncRead + AsyncWrite + Unpin + Clone + 'static> AsyncStream for T {}

pub trait Splitable {
    fn split(self) -> io::Result<(Self, Self)>
    where
        Self: Sized;
}

pub trait Connector: Send + Sync + 'static {
    type Stream: AsyncStream;
    async fn connect(&self) -> io::Result<Self::Stream>;
}

pub trait Acceptor: 'static {
    type Stream: AsyncStream;
    async fn accept(&self) -> io::Result<(Self::Stream, SockAddr)>;
}

pub trait AcceptorBuilder: Send + Sync + 'static {
    type Stream: AsyncStream;
    type Acceptor: Acceptor<Stream = Self::Stream>;
    async fn bind(self) -> io::Result<Self::Acceptor>;
}
