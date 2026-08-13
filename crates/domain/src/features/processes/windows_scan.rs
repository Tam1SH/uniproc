use std::collections::HashSet;

#[cfg(windows)]
pub fn visible_window_pids() -> HashSet<u32> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::BOOL;

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let pids = &mut *(lparam.0 as *mut HashSet<u32>);
            if IsWindowVisible(hwnd).as_bool() {
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid != 0 {
                    pids.insert(pid);
                }
            }
            BOOL(1)
        }
    }

    let mut pids = HashSet::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut pids as *mut _ as isize));
    }
    pids
}

#[cfg(not(windows))]
pub fn visible_window_pids() -> HashSet<u32> {
    HashSet::new()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn the_session_has_visible_windows() {
        let pids = visible_window_pids();

        assert!(
            !pids.is_empty(),
            "a desktop session always has at least one visible window; \
             an empty set means the caller is not in a windowed session"
        );
        assert!(!pids.contains(&0));
    }
}
