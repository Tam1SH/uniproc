use crate::align_buffer::AlignedBuffer;
use bytes::{BufMut, BytesMut};
use compio::buf::{IoBuf, IoBufMut, SetLen};
use rkyv::util::AlignedVec;
use std::cell::RefCell;
use std::mem::MaybeUninit;

const BUCKETS: [usize; 13] = [
    8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 16384, 65536, 131072,
];

pub struct InnerPool {
    headers: Vec<BytesMut>,
    body_buckets: [Vec<AlignedBuffer>; 13],
    ema_size: usize,
}

thread_local! {
    static POOL: RefCell<InnerPool> = RefCell::new(InnerPool::new());
}

pub struct TpcPool;

fn get_max_count_for_size(size: usize) -> usize {
    match size {
        0..=1024 => 128,
        1025..=16384 => 64,
        16385..=65536 => 32,
        65537..=1048576 => 8,
        _ => 0,
    }
}

impl TpcPool {
    pub fn with<F, R>(f: F) -> R
    where
        F: FnOnce(&mut InnerPool) -> R,
    {
        POOL.with(|p| f(&mut p.borrow_mut()))
    }

    #[inline]
    pub fn acquire_header() -> BytesMut {
        Self::with(|p| p.acquire_header_raw(0))
    }

    #[inline]
    pub fn release_header(buf: BytesMut) {
        Self::with(|p| p.release_header(buf))
    }

    #[inline]
    pub fn acquire_body(needed_cap: usize) -> AlignedBuffer {
        Self::with(|p| p.acquire_body(needed_cap))
    }

    #[inline]
    pub fn release_body(buf: AlignedBuffer) {
        Self::with(|p| p.release_body(buf))
    }
}

impl InnerPool {
    pub fn new() -> Self {
        let mut pool = InnerPool {
            headers: Vec::with_capacity(128),
            body_buckets: Default::default(),
            ema_size: 65536,
        };

        for _ in 0..128 {
            pool.headers.push(BytesMut::with_capacity(4));
        }

        for (idx, &size) in BUCKETS.iter().enumerate() {
            let count = get_max_count_for_size(size);
            pool.body_buckets[idx].reserve(count);

            for _ in 0..count {
                pool.body_buckets[idx].push(AlignedBuffer(AlignedVec::with_capacity(size)));
            }
        }

        pool
    }

    pub fn acquire_header_raw(&mut self, msg_len: usize) -> BytesMut {
        let mut h = self
            .headers
            .pop()
            .unwrap_or_else(|| BytesMut::with_capacity(4));
        h.clear();
        if msg_len > 0 {
            h.put_u32_le(msg_len as u32);
        }
        h
    }

    pub fn release_header(&mut self, mut buf: BytesMut) {
        if self.headers.len() < 128 {
            buf.clear();
            self.headers.push(buf);
        }
    }

    pub fn acquire_header(&mut self, len: usize) -> BytesMut {
        let mut h = self
            .headers
            .pop()
            .unwrap_or_else(|| BytesMut::with_capacity(4));
        h.clear();
        h.put_u32_le(len as u32);
        h
    }

    pub fn acquire_body(&mut self, needed_cap: usize) -> AlignedBuffer {
        let bucket_idx = BUCKETS.iter().position(|&s| s >= needed_cap);
        if let Some(idx) = bucket_idx {
            if let Some(buf) = self.body_buckets[idx].pop() {
                return buf;
            }

            return AlignedBuffer(AlignedVec::with_capacity(BUCKETS[idx]));
        }
        AlignedBuffer(AlignedVec::with_capacity(needed_cap))
    }

    pub fn release_mixed(&mut self, buf: Mixed) {
        match buf {
            Mixed::Bytes(mut h) => {
                if self.headers.len() < 128 {
                    h.clear();
                    self.headers.push(h);
                }
            }
            Mixed::AlignedBuffer(b) => self.release_body(b),
        }
    }

    pub fn release_body(&mut self, mut buf: AlignedBuffer) {
        let cap = buf.0.capacity();
        buf.0.clear();

        self.ema_size = (self.ema_size as f32 * 0.9 + cap as f32 * 0.1) as usize;
        if cap > self.ema_size * 3 && cap > 131072 {
            return;
        }

        let bucket_idx = BUCKETS.iter().rposition(|&s| s <= cap);
        if let Some(idx) = bucket_idx {
            if self.body_buckets[idx].len() < 128 {
                self.body_buckets[idx].push(buf);
            }
        }
    }
}

pub enum Mixed {
    Bytes(BytesMut),
    AlignedBuffer(AlignedBuffer),
}

impl IoBuf for Mixed {
    fn as_init(&self) -> &[u8] {
        match self {
            Mixed::Bytes(b) => b.as_init(),
            Mixed::AlignedBuffer(b) => b.as_init(),
        }
    }
}

impl SetLen for Mixed {
    unsafe fn set_len(&mut self, len: usize) {
        match self {
            Mixed::Bytes(b) => b.set_len(len),
            Mixed::AlignedBuffer(b) => b.set_len(len),
        }
    }
}

impl IoBufMut for Mixed {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        match self {
            Mixed::Bytes(b) => b.as_uninit(),
            Mixed::AlignedBuffer(b) => b.as_uninit(),
        }
    }
}
