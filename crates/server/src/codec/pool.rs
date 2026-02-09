use crate::align_buffer::AlignedBuffer;
use rkyv::util::AlignedVec;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, instrument, trace};

pub struct BufferPool {
    buffers: Mutex<Vec<AlignedBuffer>>,
}

impl BufferPool {
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            buffers: Mutex::new(Vec::with_capacity(capacity)),
        })
    }

    #[instrument(skip(self, needed_cap), level = "trace")]
    pub fn acquire(&self, needed_cap: usize) -> AlignedBuffer {
        let mut pool = self.buffers.lock().unwrap();

        let mut best_idx = None;
        let mut min_waste = usize::MAX;

        for (i, buf) in pool.iter().enumerate() {
            let cap = buf.0.capacity();
            if cap >= needed_cap {
                let waste = cap - needed_cap;
                if waste < min_waste {
                    min_waste = waste;
                    best_idx = Some(i);

                    if waste == 0 {
                        break;
                    }
                }
            }
        }

        if let Some(idx) = best_idx {
            trace!(event = "pool_hit", needed = needed_cap, waste = min_waste);

            return pool.swap_remove(idx);
        }

        trace!(event = "pool_miss_alloc", needed = needed_cap);
        AlignedBuffer(AlignedVec::with_capacity(needed_cap))
    }

    #[instrument(skip(self, buf), level = "trace", fields(cap = buf.0.capacity()))]
    pub fn release(&self, mut buf: AlignedBuffer) {
        buf.0.clear();

        let mut pool = self.buffers.lock().unwrap();

        if pool.len() < 64 {
            pool.push(buf);
            trace!(event = "pool_return", total_in_pool = pool.len());
        } else {
            debug!(event = "pool_overflow_drop", cap = buf.0.capacity());
        }
    }
}
