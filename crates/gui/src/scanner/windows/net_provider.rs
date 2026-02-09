use std::collections::HashMap;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetProcessIoCounters, OpenProcess, IO_COUNTERS,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

pub struct ProcessNetProvider {
    history: HashMap<u32, u64>,
}

impl ProcessNetProvider {
    pub fn new() -> Self {
        Self {
            history: HashMap::with_capacity(500),
        }
    }

    pub fn get_usage(&mut self, pid: u32) -> u64 {
        let current_total = self.query_io_other(pid);

        let previous_total = self.history.get(&pid).cloned().unwrap_or(current_total);

        self.history.insert(pid, current_total);

        current_total.saturating_sub(previous_total)
    }

    pub fn cleanup(&mut self, active_pids: &[u32]) {
        self.history.retain(|pid, _| active_pids.contains(pid));
    }

    fn query_io_other(&self, pid: u32) -> u64 {
        unsafe {
            let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return 0;
            };

            let mut counters = IO_COUNTERS::default();

            let res = GetProcessIoCounters(handle, &mut counters);

            let _ = CloseHandle(handle);

            if res.is_ok() {
                counters.OtherTransferCount + counters.WriteTransferCount
            } else {
                0
            }
        }
    }
}
