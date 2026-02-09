use crate::align_buffer::AlignedBuffer;
use anyhow::Result;
use compio::io::{AsyncRead, AsyncWrite};
use rkyv::api::high::{HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Source;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::{Portable, Serialize};

pub trait AsyncMessageCodec {
    async fn recv<S: AsyncRead>(&mut self, socket: &mut S) -> Result<&AlignedBuffer>;
    async fn send<T, S: AsyncWrite>(&mut self, socket: &mut S, msg: &T) -> Result<()>
    where
        T: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>;

    fn interpret<'b, T, E>(buf: &'b AlignedBuffer) -> Result<&'b T>
    where
        E: Source,
        T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>;

    fn release_recv(&mut self);
}
