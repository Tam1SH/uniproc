use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, GAA_FLAG_SKIP_UNICAST, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;

pub struct BandwidthScanner;

impl BandwidthScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn get_total_bandwidth(&self) -> u64 {
        let mut out_buf_len: u32 = 15000;
        let mut addresses: Vec<u8> = vec![0; out_buf_len as usize];

        unsafe {
            let mut ret = GetAdaptersAddresses(
                0,
                GAA_FLAG_SKIP_UNICAST,
                None,
                Some(addresses.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut out_buf_len,
            );

            if ret == ERROR_BUFFER_OVERFLOW.0 {
                addresses.resize(out_buf_len as usize, 0);
                ret = GetAdaptersAddresses(
                    0,
                    GAA_FLAG_SKIP_UNICAST,
                    None,
                    Some(addresses.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                    &mut out_buf_len,
                );
            }

            if ret != ERROR_SUCCESS.0 {
                return 0;
            }

            let mut total_bandwidth_bits = 0u64;
            let mut curr_ptr = addresses.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

            while !curr_ptr.is_null() {
                let adapter = &*curr_ptr;

                if adapter.OperStatus == IfOperStatusUp {
                    total_bandwidth_bits += adapter.ReceiveLinkSpeed;
                }

                curr_ptr = adapter.Next;
            }

            total_bandwidth_bits / 8
        }
    }
}
