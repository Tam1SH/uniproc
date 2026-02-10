use crate::server::transport::stream::Splitable;
use compio::buf::{IoBuf, IoBufMut};
use compio::io::{AsyncRead, AsyncWrite};
use compio::BufResult;
use socket2::SockAddr;
use std::io;

#[derive(Clone)]
pub enum VStream {
    #[cfg(windows)]
    Hv(crate::server::transport::stream::vsock::windows::HvStream),

    #[cfg(unix)]
    Vsock(crate::server::transport::stream::vsock::linux::VsockStream),
}

impl Splitable for VStream {
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

impl VStream {
    pub async fn connect(cid: u32, port: u32) -> io::Result<Self> {
        #[cfg(unix)]
        {
            Ok(Self::Vsock(
                crate::server::transport::stream::vsock::linux::VsockStream::connect(cid, port)
                    .await?,
            ))
        }
        #[cfg(windows)]
        {
            use crate::server::transport::stream::vsock::windows::ToServiceId;
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
            Ok(Self::Hv(
                crate::server::transport::stream::vsock::windows::HvStream::connect(vm_guid, guid)
                    .await?,
            ))
        }
    }
}

impl AsyncRead for VStream {
    async fn read<B: IoBufMut>(&mut self, buf: B) -> BufResult<usize, B> {
        match self {
            #[cfg(windows)]
            Self::Hv(s) => s.read(buf).await,

            #[cfg(unix)]
            Self::Vsock(s) => s.read(buf).await,
        }
    }
}

impl AsyncWrite for VStream {
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

pub enum VListener {
    #[cfg(windows)]
    Hv(crate::server::transport::stream::vsock::windows::HvListener),
    #[cfg(unix)]
    Vsock(crate::server::transport::stream::vsock::linux::VsockListener),
}

impl VListener {
    pub fn bind(port: u32) -> io::Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self::Hv(
                crate::server::transport::stream::vsock::windows::HvListener::bind(port)?,
            ))
        }
        #[cfg(unix)]
        {
            Ok(Self::Vsock(
                crate::server::transport::stream::vsock::linux::VsockListener::bind(port)?,
            ))
        }
    }

    pub async fn accept(&self) -> io::Result<(VStream, SockAddr)> {
        match self {
            #[cfg(windows)]
            Self::Hv(l) => {
                let (stream, addr) = l.accept().await?;
                Ok((VStream::Hv(stream), addr))
            }
            #[cfg(unix)]
            Self::Vsock(l) => Ok(VStream::Vsock(l.accept().await?.0)),
        }
    }
}
