use compio::buf::{IntoInner, IoBuf, IoBufMut};
use compio::driver::op::{Accept, Connect, Recv};
use compio::io::{AsyncRead, AsyncWrite};
use compio::runtime::{submit, Attacher};
use compio::BufResult;
use socket2::{Domain, SockAddr, Socket, Type};
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd};

pub struct VsockStream {
    inner: Attacher<OwnedFd>,
}

impl VsockStream {
    pub async fn connect(cid: u32, port: u32) -> io::Result<Self> {
        let socket = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
        let addr = SockAddr::vsock(cid, port);

        let op = Connect::new(socket.as_raw_fd(), addr);
        let BufResult(res, _) = submit(op).await;
        res?;

        Ok(Self {
            inner: Attacher::new(OwnedFd::from(socket))?,
        })
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let fd = self.inner.as_raw_fd();
        let new_fd = unsafe { libc::dup(fd) };
        if new_fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Self::from_raw(new_fd)
    }

    pub fn from_raw(fd: i32) -> io::Result<Self> {
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        Ok(Self {
            inner: Attacher::new(owned)?,
        })
    }
}

impl AsyncRead for VsockStream {
    async fn read<B: IoBufMut>(&mut self, buf: B) -> BufResult<usize, B> {
        let op = Recv::new(self.inner.as_raw_fd(), buf, 0);
        submit(op).await.map_buffer(|op| op.into_inner())
    }
}

impl AsyncWrite for VsockStream {
    async fn write<T: IoBuf>(&mut self, buf: T) -> BufResult<usize, T> {
        let op = Send::new(self.inner.as_raw_fd(), buf, 0);
        submit(op).await.map_buffer(|op| op.into_inner())
    }
    async fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> io::Result<()> {
        unsafe {
            libc::shutdown(self.inner.as_raw_fd(), libc::SHUT_WR);
        }
        Ok(())
    }
}

pub struct VsockListener {
    inner: Attacher<OwnedFd>,
}

impl VsockListener {
    pub fn bind(port: u32) -> io::Result<Self> {
        let socket = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
        let addr = SockAddr::vsock(libc::VMADDR_CID_ANY, port);
        socket.bind(&addr)?;
        socket.listen(128)?;
        Ok(Self {
            inner: Attacher::new(OwnedFd::from(socket))?,
        })
    }

    pub async fn accept(&self) -> io::Result<(VsockStream, SockAddr)> {
        let accept_socket = Socket::new(Domain::VSOCK, Type::STREAM, None)?;
        let op = Accept::new(self.inner.as_raw_fd());
        let BufResult(res, op) = submit(op).await;
        res?;
        let (socket, addr) = op.into_addr()?;
        Ok((VsockStream::from_raw(socket.as_raw_fd())?, addr))
    }
}
