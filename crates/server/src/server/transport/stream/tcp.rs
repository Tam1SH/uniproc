use crate::server::transport::stream::{Acceptor, AcceptorBuilder, Connector, Splitable};
use compio::net::{TcpListener, TcpStream};
use socket2::SockAddr;
use std::io;
use std::net::SocketAddr;

impl Acceptor for TcpListener {
    type Stream = TcpStream;

    async fn accept(&self) -> io::Result<(Self::Stream, SockAddr)> {
        self.accept().await.map(|(s, a)| (s, a.into()))
    }
}

impl Splitable for TcpStream {
    fn split(self) -> io::Result<(Self, Self)> {
        Ok((self.clone(), self))
    }
}

pub struct TcpAcceptorBuilder {
    pub addr: SocketAddr,
}

impl TcpAcceptorBuilder {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

impl AcceptorBuilder for TcpAcceptorBuilder {
    type Stream = TcpStream;
    type Acceptor = TcpListener;

    async fn bind(self) -> io::Result<Self::Acceptor> {
        TcpListener::bind(self.addr).await
    }
}

pub struct TcpConnector {
    pub addr: SocketAddr,
}

impl TcpConnector {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

impl Connector for TcpConnector {
    type Stream = TcpStream;

    async fn connect(&self) -> io::Result<Self::Stream> {
        TcpStream::connect(self.addr).await
    }
}
