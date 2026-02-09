pub mod traits;

pub mod pool;

use crate::codec::pool::BufferPool;
use crate::codec::traits::AsyncMessageCodec;
use anyhow::{anyhow, Result};
use compio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::align_buffer::AlignedBuffer;
use compio::BufResult;
use rkyv::api::high::{HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::{Error, Source};
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::{access, Portable, Serialize};
use std::sync::Arc;

pub struct Codec {
    pub(crate) pool: Arc<BufferPool>,

    current_recv: Option<AlignedBuffer>,

    header_buf: Option<Vec<u8>>,
}

impl Clone for Codec {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            current_recv: None,
            header_buf: None,
        }
    }
}

impl Codec {
    pub fn new(pool: Arc<BufferPool>) -> Self {
        Self {
            pool,
            current_recv: None,
            header_buf: Some(vec![0u8; 4]),
        }
    }
}

impl AsyncMessageCodec for Codec {
    async fn recv<S: AsyncRead>(&mut self, socket: &mut S) -> Result<&AlignedBuffer> {
        let h_buf = self.header_buf.take().unwrap_or_else(|| vec![0u8; 4]);

        let BufResult(res, h_buf_back) = socket.read_exact(h_buf).await;

        if let Err(e) = res {
            self.header_buf = Some(h_buf_back);
            return Err(e.into());
        }

        let len = u32::from_le_bytes(h_buf_back[..4].try_into().unwrap()) as usize;
        self.header_buf = Some(h_buf_back);

        let mut buf = self.pool.acquire(len);
        buf.0.resize(len, 0);

        let BufResult(res, buf) = socket.read_exact(buf).await;

        if let Err(e) = res {
            self.pool.release(buf);
            return Err(e.into());
        }

        self.current_recv = Some(buf);
        Ok(self.current_recv.as_ref().unwrap())
    }

    async fn send<T, S: AsyncWrite>(&mut self, socket: &mut S, msg: &T) -> Result<()>
    where
        T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, Error>>,
    {
        let bytes = rkyv::to_bytes::<Error>(msg)
            .map_err(|e| anyhow!("rkyv serialization failed: {:?}", e))?;

        let len = bytes.len() as u32;
        let len_bytes = len.to_le_bytes();

        socket.write_all(len_bytes.to_vec()).await.0?;

        let BufResult(res, bytes) = socket.write_all(bytes.into_vec()).await;
        res.map_err(|e| anyhow!("Socket write error: {}", e))?;

        Ok(())
    }
    fn interpret<'b, T, E>(buf: &'b AlignedBuffer) -> Result<&'b T>
    where
        E: Source,
        T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>,
    {
        let archived = access::<T, E>(buf.0.as_slice())
            .map_err(|e| anyhow!("Rkyv validation error: {:?}", e))?;

        Ok(archived)
    }

    fn release_recv(&mut self) {
        if let Some(buf) = self.current_recv.take() {
            self.pool.release(buf);
        }
    }
}
