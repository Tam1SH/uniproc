use std::collections::HashMap;
use std::ptr::{addr_of, null_mut};
use windows::core::{w, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{FALSE, HWND, LPARAM};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::Storage::Packaging::Appx::{
    PackageIdFromFullName, PACKAGE_ID, PACKAGE_INFORMATION_BASIC,
};
use windows::Win32::System::Services::{
    EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW, SC_ENUM_PROCESS_INFO,
    SC_MANAGER_ENUMERATE_SERVICE, SERVICE_ACTIVE, SERVICE_WIN32,
};
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHLoadIndirectString, SHFILEINFOW, SHGFI_DISPLAYNAME, SHGFI_USEFILEATTRIBUTES,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

pub fn get_active_services_map() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    unsafe {
        let Ok(scm) = OpenSCManagerW(None, None, SC_MANAGER_ENUMERATE_SERVICE) else {
            return map;
        };

        let mut bytes_needed = 0;
        let mut services_returned = 0;
        let mut resume_handle = 0;

        let _ = EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_ACTIVE,
            None,
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            None,
        );

        let mut buffer = vec![0u8; bytes_needed as usize];

        if EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_ACTIVE,
            Some(buffer.as_mut_slice()),
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            None,
        )
        .is_ok()
        {
            let services_ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
            let services_slice =
                std::slice::from_raw_parts(services_ptr, services_returned as usize);

            for svc in services_slice {
                let pid = svc.ServiceStatusProcess.dwProcessId;
                if pid > 0
                    && let Ok(name) = svc.lpDisplayName.to_string() {
                        map.entry(pid)
                            .and_modify(|e| {
                                e.push_str(" / ");
                                e.push_str(&name);
                            })
                            .or_insert(name);
                    }
            }
        }
    }
    map
}

struct EnumCtx {
    map: HashMap<u32, String>,
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL { unsafe {
    if IsWindowVisible(hwnd).as_bool() {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid > 0 {
            let ctx = &mut *(lparam.0 as *mut EnumCtx);
            if let std::collections::hash_map::Entry::Vacant(e) = ctx.map.entry(pid) {
                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut buf = vec![0u16; (len + 1) as usize];
                    if GetWindowTextW(hwnd, &mut buf) > 0
                        && let Ok(title) = String::from_utf16(&buf[..len as usize]) {
                            e.insert(title);
                        }
                }
            }
        }
    }
    BOOL::from(true)
}}

pub fn get_visible_windows_map() -> HashMap<u32, String> {
    let mut ctx = EnumCtx {
        map: HashMap::new(),
    };
    unsafe {
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.map
}

pub fn get_package_display_name(package_full_name: &str) -> Option<String> {
    let h_full_name = HSTRING::from(package_full_name);
    let mut buffer_size = 0u32;

    unsafe {
        let _ = PackageIdFromFullName(
            PCWSTR(h_full_name.as_ptr()),
            PACKAGE_INFORMATION_BASIC,
            &mut buffer_size,
            None,
        );
    }

    if buffer_size == 0 {
        return None;
    }

    let mut buffer = vec![0u8; buffer_size as usize];
    unsafe {
        if PackageIdFromFullName(
            PCWSTR(h_full_name.as_ptr()),
            PACKAGE_INFORMATION_BASIC,
            &mut buffer_size,
            Some(buffer.as_mut_ptr()),
        )
        .is_err()
        {
            return None;
        }

        let pkg_id_ptr = buffer.as_ptr() as *const PACKAGE_ID;

        let name_pwstr: PWSTR = addr_of!((*pkg_id_ptr).name).read_unaligned();

        let base_name = name_pwstr.to_string().ok()?;

        let indirect_path = format!(
            "@{{{0}?ms-resource://{1}/resources/AppName}}",
            package_full_name, base_name
        );
        let h_indirect = HSTRING::from(&indirect_path);
        let mut out_buffer = [0u16; 256];

        if SHLoadIndirectString(PCWSTR(h_indirect.as_ptr()), &mut out_buffer, None).is_ok() {
            let resolved = String::from_utf16_lossy(&out_buffer)
                .trim_matches(char::from(0))
                .trim()
                .to_string();

            if !resolved.is_empty() && !resolved.starts_with('@') {
                return Some(resolved);
            }
        }

        None
    }
}

pub fn get_win32_description(exe_path: &str) -> Option<String> {
    unsafe {
        let path_h = HSTRING::from(exe_path);
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(PCWSTR(path_h.as_ptr()), Some(&mut handle));
        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        if GetFileVersionInfoW(
            PCWSTR(path_h.as_ptr()),
            None,
            size,
            buffer.as_mut_ptr() as *mut _,
        )
        .is_err()
        {
            return None;
        }

        let mut lp_translate = null_mut();
        let mut cb_translate = 0u32;

        if VerQueryValueW(
            buffer.as_ptr() as *const _,
            w!("\\VarFileInfo\\Translation"),
            &mut lp_translate,
            &mut cb_translate,
        ) == FALSE
        {
            return None;
        }

        let lang = *(lp_translate as *const u32);
        let sub_block = format!(
            "\\StringFileInfo\\{:04x}{:04x}\\FileDescription",
            lang & 0xFFFF,
            (lang >> 16) & 0xFFFF
        );
        let sub_block_h = HSTRING::from(&sub_block);

        let mut desc_ptr = std::ptr::null_mut();
        let mut desc_len = 0u32;
        if VerQueryValueW(
            buffer.as_ptr() as *const _,
            PCWSTR(sub_block_h.as_ptr()),
            &mut desc_ptr,
            &mut desc_len,
        ) != FALSE
            && let Ok(desc) = PWSTR(desc_ptr as *mut _).to_string() {
                let trimmed = desc.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
    }
    None
}

pub fn get_shell_name(name: &str) -> Option<String> {
    let mut shfi = SHFILEINFOW::default();
    let name_h = HSTRING::from(name);
    unsafe {
        SHGetFileInfoW(
            PCWSTR(name_h.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_DISPLAYNAME | SHGFI_USEFILEATTRIBUTES,
        );
        let res = String::from_utf16_lossy(&shfi.szDisplayName)
            .trim_matches(char::from(0))
            .to_string();
        if res.is_empty() { None } else { Some(res) }
    }
}
