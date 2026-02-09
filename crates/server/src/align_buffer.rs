use compio::buf::{IoBuf, IoBufMut, SetLen};
use rkyv::util::AlignedVec;
use std::mem::MaybeUninit;

#[derive(Default)]
pub struct AlignedBuffer(pub AlignedVec);

impl IoBuf for AlignedBuffer {
    fn as_init(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl SetLen for AlignedBuffer {
    unsafe fn set_len(&mut self, len: usize) {
        unsafe { self.0.set_len(len) };
    }
}

impl IoBufMut for AlignedBuffer {
    fn as_uninit(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe {
            let ptr = self.0.as_mut_ptr() as *mut MaybeUninit<u8>;

            std::slice::from_raw_parts_mut(ptr, self.0.len())
        }
    }
}
