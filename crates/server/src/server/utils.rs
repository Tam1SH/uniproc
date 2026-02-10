use tracing::{debug, warn};

pub fn current_cpu() -> usize {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::sched_getcpu() as usize
    }

    #[cfg(target_os = "windows")]
    unsafe {
        windows::Win32::System::Threading::GetCurrentProcessorNumber() as usize
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    0 // fallback
}

pub fn set_thread_high_priority() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Threading::*;
        let thread_handle = GetCurrentThread();
        if SetThreadPriority(thread_handle, THREAD_PRIORITY_HIGHEST).is_err() {
            warn!("Failed to set Windows thread priority");
        } else {
            debug!("Windows thread priority set to HIGHEST");
        }
    }

    #[cfg(unix)]
    unsafe {
        let thread_id = libc::pthread_self();
        let mut param: libc::sched_param = std::mem::zeroed();
        param.sched_priority = 10;

        let res = libc::pthread_setschedparam(thread_id, libc::SCHED_FIFO, &param);
        if res != 0 {
            if libc::setpriority(libc::PRIO_PROCESS, 0, -20) != 0 {
                warn!("Failed to set Linux thread priority (try sudo or ulimit -r)");
            } else {
                debug!("Linux thread priority set via setpriority (nice -20)");
            }
        } else {
            debug!("Linux thread priority set to SCHED_FIFO");
        }
    }
}
