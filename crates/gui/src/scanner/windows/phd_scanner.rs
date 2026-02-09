use std::ptr::null_mut;
use windows::Win32::System::Performance::*;

pub struct PdhScanner;

impl PdhScanner {
    pub fn get_disk_percent(&self) -> f32 {
        unsafe {
            let mut query = PDH_HQUERY(null_mut());
            PdhOpenQueryW(None, 0, &mut query);

            let mut disk_counter = PDH_HCOUNTER(null_mut());

            PdhAddCounterW(
                query,
                windows::core::w!("\\PhysicalDisk(_Total)\\% Disk Time"),
                0,
                &mut disk_counter,
            );

            PdhCollectQueryData(query);

            let mut disk_val = Default::default();
            PdhGetFormattedCounterValue(disk_counter, PDH_FMT_DOUBLE, None, &mut disk_val);

            (disk_val.Anonymous.doubleValue as f32).clamp(0.0, 100.0)
        }
    }
}
