use windows::Wdk::System::Threading::{NtQueryInformationProcess, PROCESSINFOCLASS};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

pub const PROCESS_TIMES_INFORMATION: i32 = 4;

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Debug, Default, Copy, Clone)]
pub struct KERNEL_USER_TIMES {
    pub CreateTime: i64,
    pub ExitTime: i64,
    pub KernelTime: i64,
    pub UserTime: i64,
}

pub struct ProcessSpotlight {
    handle: HANDLE,
    last_process_time: i64,
    last_system_time: i64,
}

impl ProcessSpotlight {
    pub fn new(pid: u32) -> Option<Self> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            Some(Self {
                handle,
                last_process_time: 0,
                last_system_time: 0,
            })
        }
    }

    pub fn sample(&mut self) -> f32 {
        unsafe {
            let mut times = KERNEL_USER_TIMES::default();
            let mut ret_len = 0;

            let status = NtQueryInformationProcess(
                self.handle,
                PROCESSINFOCLASS(PROCESS_TIMES_INFORMATION),
                &mut times as *mut _ as _,
                std::mem::size_of::<KERNEL_USER_TIMES>() as u32,
                &mut ret_len,
            );

            if status.is_err() {
                return 0.0;
            }

            let mut sys_idle = 0i64;
            let mut sys_kernel = 0i64;
            let mut sys_user = 0i64;

            let current_process_time = times.KernelTime + times.UserTime;
            let current_system_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64;

            let proc_delta = current_process_time - self.last_process_time;
            let sys_delta = current_system_time - self.last_system_time;

            self.last_process_time = current_process_time;
            self.last_system_time = current_system_time;

            if sys_delta <= 0 {
                return 0.0;
            }

            (proc_delta as f32 / sys_delta as f32) * 100.0
        }
    }
}

pub fn get_process_cpu_usage(handle: windows::Win32::Foundation::HANDLE) -> f32 {
    let mut times = KERNEL_USER_TIMES::default();
    let mut ret_len = 0u32;

    unsafe {
        let status = NtQueryInformationProcess(
            handle,
            PROCESSINFOCLASS(PROCESS_TIMES_INFORMATION),
            &mut times as *mut _ as _,
            size_of::<KERNEL_USER_TIMES>() as u32,
            &mut ret_len,
        );

        if status.is_err() {
            return 0.0;
        }
    }

    (times.KernelTime + times.UserTime) as f32
}
//
// impl Drop for ProcessSpotlight {
//     fn drop(&mut self) {
//         unsafe {
//             let _ = CloseHandle(self.handle);
//         }
//     }
// }
