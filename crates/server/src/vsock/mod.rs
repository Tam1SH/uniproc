#[cfg(unix)]
pub mod linux;
#[cfg(windows)]
pub mod windows;

use compio::buf::{IoBuf, IoBufMut};
use compio::io::{AsyncRead, AsyncWrite};
use compio::net::{TcpListener, TcpStream};
use compio::BufResult;
use socket2::SockAddr;
use std::io;

pub trait AsyncAcceptor: 'static {
    type Stream: AsyncStream + Clone;

    async fn accept(&self) -> io::Result<Self::Stream>;
}

impl AsyncAcceptor for Listener {
    type Stream = Stream;

    async fn accept(&self) -> io::Result<Self::Stream> {
        self.accept().await.map(|(stream, _)| stream)
    }
}

impl AsyncAcceptor for TcpListener {
    type Stream = TcpStream;

    async fn accept(&self) -> io::Result<Self::Stream> {
        self.accept().await.map(|(stream, _)| stream)
    }
}

pub trait AsyncStream: Splitable + AsyncRead + AsyncWrite + Unpin + 'static {}

impl<T: Splitable + AsyncRead + AsyncWrite + Unpin + 'static> AsyncStream for T {}

#[derive(Clone)]
pub enum Stream {
    #[cfg(windows)]
    Hv(windows::HvStream),

    #[cfg(unix)]
    Vsock(linux::VsockStream),
}

pub trait Splitable {
    fn split(self) -> io::Result<(Self, Self)>
    where
        Self: Sized;
}

impl Splitable for Stream {
    fn split(self) -> io::Result<(Self, Self)> {
        let clone = match &self {
            #[cfg(windows)]
            Self::Hv(s) => Self::Hv(s.clone()),
            #[cfg(unix)]
            Self::Vsock(s) => Self::Vsock(s.try_clone()?),
        };
        Ok((clone, self))
    }
}

impl Splitable for TcpStream {
    fn split(self) -> io::Result<(Self, Self)> {
        Ok((self.clone(), self))
    }
}

impl Stream {
    pub async fn connect(cid: u32, port: u32) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self::Vsock(linux::VsockStream::connect(cid, port).await?))
        }
        #[cfg(windows)]
        {
            use crate::vsock::windows::ToServiceId;

            let vm_guid = match cid {
                0 | 1 => ::windows::Win32::System::Hypervisor::HV_GUID_LOOPBACK,
                2 => ::windows::Win32::System::Hypervisor::HV_GUID_PARENT,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Unsupported CID on Windows",
                    ));
                }
            };

            let guid = port.to_guid();
            Ok(Self::Hv(windows::HvStream::connect(vm_guid, guid).await?))
        }
    }
}

impl AsyncRead for Stream {
    async fn read<B: IoBufMut>(&mut self, buf: B) -> BufResult<usize, B> {
        match self {
            #[cfg(windows)]
            Self::Hv(s) => s.read(buf).await,

            #[cfg(unix)]
            Self::Vsock(s) => s.read(buf).await,
        }
    }
}

impl AsyncWrite for Stream {
    async fn write<T: IoBuf>(&mut self, buf: T) -> BufResult<usize, T> {
        match self {
            #[cfg(windows)]
            Self::Hv(s) => s.write(buf).await,

            #[cfg(unix)]
            Self::Vsock(s) => s.write(buf).await,
        }
    }

    async fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(windows)]
            Self::Hv(s) => s.flush().await,

            #[cfg(unix)]
            Self::Vsock(s) => s.flush().await,
        }
    }

    async fn shutdown(&mut self) -> io::Result<()> {
        match self {
            #[cfg(windows)]
            Self::Hv(s) => s.shutdown().await,

            #[cfg(unix)]
            Self::Vsock(s) => s.shutdown().await,
        }
    }
}

pub enum Listener {
    #[cfg(windows)]
    Hv(windows::HvListener),
    #[cfg(unix)]
    Vsock(linux::VsockListener),
}

impl Listener {
    pub fn bind(port: u32) -> io::Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self::Hv(windows::HvListener::bind(port)?))
        }
        #[cfg(unix)]
        {
            Ok(Self::Vsock(linux::VsockListener::bind(port)?))
        }
    }

    pub async fn accept(&self) -> io::Result<(Stream, SockAddr)> {
        match self {
            #[cfg(windows)]
            Self::Hv(l) => {
                let (stream, addr) = l.accept().await?;
                Ok((Stream::Hv(stream), addr))
            }
            #[cfg(unix)]
            Self::Vsock(l) => Ok(Stream::Vsock(l.accept().await?.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compio::io::{AsyncReadExt, AsyncWriteExt};

    const TEST_PORT: u32 = 12345;

    #[compio::test]
    async fn test_stream_full_cycle() {
        let listener = Listener::bind(TEST_PORT).expect("Failed to bind listener");

        let client_task = async {
            let mut client = Stream::connect(1, TEST_PORT)
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
        let listener = Listener::bind(TEST_PORT + 1).expect("Bind failed");

        let client_fut = async {
            let stream = Stream::connect(1, TEST_PORT + 1).await.unwrap();
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
