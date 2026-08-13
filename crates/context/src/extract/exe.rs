use windows::core::PCWSTR;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use super::bitmap::{hicon_to_rgba, RgbaImage};

pub fn extract_icon_rgba(path: &str) -> Option<RgbaImage> {
    if !has_own_icon(path) {
        return None;
    }

    unsafe {
        let mut shfi: SHFILEINFOW = std::mem::zeroed();
        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        let result = SHGetFileInfoW(
            PCWSTR(path_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );

        if result == 0 || shfi.hIcon.is_invalid() {
            return None;
        }

        let img = hicon_to_rgba(shfi.hIcon);
        let _ = DestroyIcon(shfi.hIcon);
        img
    }
}

pub fn has_own_icon(exe_path: &str) -> bool {
    let mut buffer = [0u16; 260];
    for (i, wide_char) in exe_path.encode_utf16().enumerate() {
        if i >= 259 {
            break;
        }
        buffer[i] = wide_char;
    }

    unsafe { PrivateExtractIconsW(&buffer, 0, 0, 0, None, None, 0) > 0 }
}
