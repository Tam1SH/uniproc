use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn extract_icon_raw(path: &str) -> Option<Image> {
    unsafe {
        let mut shfi: SHFILEINFOW = std::mem::zeroed();
        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        let result = SHGetFileInfoW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );

        if result == 0 || shfi.hIcon.is_invalid() {
            return None;
        }

        let img = hicon_to_slint_image(shfi.hIcon);
        let _ = DestroyIcon(shfi.hIcon);
        img
    }
}

unsafe fn hicon_to_slint_image(hicon: HICON) -> Option<Image> {
    let mut icon_info = ICONINFO::default();

    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return None;
    }

    let _color_guard = ScopeGuard(icon_info.hbmColor);
    let _mask_guard = ScopeGuard(icon_info.hbmMask);

    let hdc = GetDC(Option::from(HWND(std::ptr::null_mut())));

    let mut bm = BITMAP::default();

    GetObjectW(
        icon_info.hbmColor.into(),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bm as *mut _ as *mut _),
    );

    let w = bm.bmWidth;
    let h = bm.bmHeight.abs();

    let mut buffer = vec![0u8; (w * h * 4) as usize];

    let mut bmi = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: 0,
        ..std::mem::zeroed()
    };

    GetDIBits(
        hdc,
        icon_info.hbmColor,
        0,
        h as u32,
        Some(buffer.as_mut_ptr() as *mut _),
        &mut bmi as *mut _ as *mut _,
        DIB_RGB_COLORS,
    );

    ReleaseDC(Option::from(HWND(std::ptr::null_mut())), hdc);

    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2);
        if chunk[3] == 0 {
            chunk[3] = 255;
        }
    }

    let pixel_buffer =
        SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&buffer, w as u32, h as u32);
    Some(Image::from_rgba8(pixel_buffer))
}

struct ScopeGuard(windows::Win32::Graphics::Gdi::HBITMAP);
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                DeleteObject(self.0.into());
            }
        }
    }
}
