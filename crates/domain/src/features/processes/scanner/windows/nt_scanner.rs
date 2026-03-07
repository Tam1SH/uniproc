use windows::Wdk::System::SystemInformation::{
    NtQuerySystemInformation, SystemProcessorPerformanceInformation,
};
use windows::Win32::Foundation::NTSTATUS;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProcessorPerformanceInfo {
    pub idle_time: i64,
    pub kernel_time: i64,
    pub user_time: i64,
    pub dpc_time: i64,
    pub interrupt_time: i64,
    pub interrupt_count: u32,
}

pub struct NtScanner {
    previous_cpu_total: i64,
    previous_cpu_idle: i64,
    core_count: usize,
}

impl NtScanner {
    pub fn new() -> Self {
        let core_count = std::thread::available_parallelism().unwrap().get();

        Self {
            previous_cpu_total: 0,
            previous_cpu_idle: 0,
            core_count,
        }
    }

    pub fn get_global_cpu(&mut self) -> f32 {
        let size = std::mem::size_of::<SystemProcessorPerformanceInfo>() * self.core_count;
        let mut buffer: Vec<SystemProcessorPerformanceInfo> =
            vec![Default::default(); self.core_count];
        let mut return_length: u32 = 0;

        unsafe {
            let status = NtQuerySystemInformation(
                SystemProcessorPerformanceInformation,
                buffer.as_mut_ptr() as *mut _,
                size as u32,
                &mut return_length,
            );

            if status != NTSTATUS(0) {
                return 0.0;
            }
        }

        let mut current_idle: i64 = 0;
        let mut current_kernel: i64 = 0;
        let mut current_user: i64 = 0;

        for info in buffer {
            current_idle += info.idle_time;
            current_kernel += info.kernel_time;
            current_user += info.user_time;
        }

        let current_total = current_kernel + current_user;

        let delta_total = current_total - self.previous_cpu_total;
        let delta_idle = current_idle - self.previous_cpu_idle;

        self.previous_cpu_total = current_total;
        self.previous_cpu_idle = current_idle;

        if delta_total == 0 {
            return 0.0;
        }

        let usage = 1.0 - (delta_idle as f32 / delta_total as f32);
        (usage * 100.0).clamp(0.0, 100.0)
    }

    pub fn get_global_ram(&self) -> (u64, u64) {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

        let mut mem_status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        unsafe {
            let _ = GlobalMemoryStatusEx(&mut mem_status);
        }

        (mem_status.ullTotalPhys, mem_status.ullAvailPhys)
    }
}
