use crate::align_buffer::AlignedBuffer;
use bytes::BytesMut;
use compio::buf::IoBufMut;
use rkyv::util::AlignedVec;
use std::cell::RefCell;

thread_local! {

    static HEADER_POOL: RefCell<Vec<BytesMut>> = RefCell::new(Vec::with_capacity(128));

    static BODY_POOL: RefCell<Vec<AlignedBuffer>> = RefCell::new(Vec::with_capacity(64));
}

pub struct TpcPool;

impl TpcPool {
    pub fn acquire_header() -> BytesMut {
        HEADER_POOL.with(|p| {
            p.borrow_mut()
                .pop()
                .unwrap_or_else(|| BytesMut::with_capacity(4))
        })
    }

    pub fn release_header(mut buf: BytesMut) {
        if buf.buf_capacity() >= 4 {
            buf.truncate(0);
            HEADER_POOL.with(|p| {
                let mut pool = p.borrow_mut();
                if pool.len() < 128 {
                    pool.push(buf);
                }
            });
        }
    }

    pub fn acquire_body(needed_cap: usize) -> AlignedBuffer {
        BODY_POOL.with(|p| {
            let mut pool = p.borrow_mut();

            if let Some(pos) = pool.iter().rposition(|b| b.0.capacity() >= needed_cap) {
                pool.swap_remove(pos)
            } else {
                AlignedBuffer(AlignedVec::with_capacity(needed_cap))
            }
        })
    }

    pub fn release_body(mut buf: AlignedBuffer) {
        buf.0.clear();
        BODY_POOL.with(|p| {
            let mut pool = p.borrow_mut();
            if pool.len() < 64 {
                pool.push(buf);
            }
        })
    }
}
