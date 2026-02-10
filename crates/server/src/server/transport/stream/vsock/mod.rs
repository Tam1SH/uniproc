pub mod general;
#[cfg(unix)]
pub mod linux;
#[cfg(windows)]
pub mod windows;

use crate::server::transport::stream::vsock::general::{VListener, VStream};
use crate::server::transport::stream::{Acceptor, AcceptorBuilder, Connector, Splitable};
use compio::buf::{IoBuf, IoBufMut};
use compio::io::{AsyncRead, AsyncWrite};
use compio::BufResult;
use socket2::SockAddr;
use std::io;

pub struct VsockConnector {
    pub cid: u32,
    pub port: u32,
}

impl VsockConnector {
    pub fn new(cid: u32, port: u32) -> Self {
        Self { cid, port }
    }
}

impl Connector for VsockConnector {
    type Stream = VStream;

    async fn connect(&self) -> io::Result<Self::Stream> {
        VStream::connect(self.cid, self.port)
            .await
            .map_err(|e| e.into())
    }
}

impl Acceptor for VListener {
    type Stream = VStream;
    async fn accept(&self) -> io::Result<(Self::Stream, SockAddr)> {
        Ok(self.accept().await?)
    }
}

pub struct VsockAcceptorBuilder {
    cid: u32,
    pub port: u32,
}

impl VsockAcceptorBuilder {
    pub fn new(cid: u32, port: u32) -> Self {
        Self { port, cid }
    }
}

impl AcceptorBuilder for VsockAcceptorBuilder {
    type Stream = VStream;
    type Acceptor = VListener;

    async fn bind(self) -> io::Result<Self::Acceptor> {
        VListener::bind(self.port).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compio::io::{AsyncReadExt, AsyncWriteExt};

    const TEST_PORT: u32 = 12345;

    #[compio::test]
    async fn test_stream_full_cycle() {
        let listener = VListener::bind(TEST_PORT).expect("Failed to bind listener");

        let client_task = async {
            let mut client = VStream::connect(1, TEST_PORT)
                .await
                .expect("Client failed to connect");

            let msg = b"hello from client";
            let BufResult(res, _) = client.write_all(msg).await;
            res.expect("Client write failed");

            let buf = vec![0u8; 17];
            let BufResult(res, buf) = client.read_exact(buf).await;
            res.expect("Client read failed");
            assert_eq!(&buf, b"hello from server");
        };

        let server_task = async {
            let (mut server_stream, _) = listener.accept().await.expect("Accept failed");

            let buf = vec![0u8; 17];
            let BufResult(res, buf) = server_stream.read_exact(buf).await;
            res.expect("Server read failed");
            assert_eq!(&buf, b"hello from client");

            let BufResult(res, _) = server_stream.write_all(b"hello from server").await;
            res.expect("Server write failed");
        };

        futures::join!(client_task, server_task);
    }

    #[compio::test]
    async fn test_stream_split() {
        let listener = VListener::bind(TEST_PORT + 1).expect("Bind failed");

        let client_fut = async {
            let stream = VStream::connect(1, TEST_PORT + 1).await.unwrap();
            let (mut reader, mut writer) = stream.split().expect("Split failed");

            writer.write_all(b"ping").await.0.unwrap();
            let buf = vec![0u8; 4];
            let BufResult(res, buf) = reader.read_exact(buf).await;
            res.expect("Split failed");
            assert_eq!(&buf, b"pong");
        };

        let server_fut = async {
            let (mut server, _) = listener.accept().await.unwrap();
            let buf = vec![0u8; 4];
            let BufResult(res, buf) = server.read_exact(buf).await;
            assert_eq!(&buf, b"ping");
            server.write_all(b"pong").await.0.unwrap();
        };

        futures::join!(client_fut, server_fut);
    }
}
