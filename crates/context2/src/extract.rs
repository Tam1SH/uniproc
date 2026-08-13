use std::ptr::null_mut;
use windows::core::{Interface, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Packaging::Appx::{
    ClosePackageInfo, GetPackageApplicationIds, PackageFamilyNameFromFullName,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub struct RgbaImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn extract_icon_rgba(path: &str) -> Option<RgbaImage> {
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

        if result == 0 || shfi.hIcon.is_invalid() || !has_own_icon(path) {
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

unsafe fn hicon_to_rgba(hicon: HICON) -> Option<RgbaImage> {
    unsafe {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }
        let _color_guard = GdiObjectGuard(icon_info.hbmColor);
        let _mask_guard = GdiObjectGuard(icon_info.hbmMask);

        let hdc = GetDC(Option::from(HWND(std::ptr::null_mut())));

        let mut bm = BITMAP::default();
        let _ = GetObjectW(
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

        let _ = GetDIBits(
            hdc,
            icon_info.hbmColor,
            0,
            h as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );

        let has_alpha = bgra_swap_and_detect_alpha(&mut buffer);
        if !has_alpha {
            match read_and_mask(hdc, icon_info.hbmMask, w, h) {
                Some(mask) => apply_and_mask(&mut buffer, &mask, w, h),
                None => {
                    for chunk in buffer.chunks_exact_mut(4) {
                        chunk[3] = 255;
                    }
                }
            }
        }

        let _ = ReleaseDC(Option::from(HWND(std::ptr::null_mut())), hdc);

        Some(RgbaImage {
            pixels: buffer,
            width: w as u32,
            height: h as u32,
        })
    }
}

unsafe fn read_and_mask(hdc: HDC, hbmp: HBITMAP, w: i32, h: i32) -> Option<Vec<u8>> {
    unsafe {
        let stride = (((w + 31) / 32) * 4) as usize;
        let mut buffer = vec![0u8; stride * h as usize];
        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 1,
            biCompression: BI_RGB.0 as u32,
            ..std::mem::zeroed()
        };
        let scan_lines = GetDIBits(
            hdc,
            hbmp,
            0,
            h as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );
        if scan_lines == 0 {
            return None;
        }
        Some(buffer)
    }
}

fn apply_and_mask(buffer: &mut [u8], mask: &[u8], w: i32, h: i32) {
    let stride = (((w + 31) / 32) * 4) as usize;
    for y in 0..h as usize {
        for x in 0..w as usize {
            let Some(&byte) = mask.get(y * stride + x / 8) else {
                continue;
            };
            let transparent = (byte >> (7 - (x % 8))) & 1 == 1;
            let idx = (y * w as usize + x) * 4;
            buffer[idx + 3] = if transparent { 0 } else { 255 };
        }
    }
}

pub fn extract_appx_icon_rgba(package_full_name: &str, size: i32) -> Option<RgbaImage> {
    unsafe {
        let h_full_name = HSTRING::from(package_full_name);
        let package_info_ref = null_mut();

        let mut len = 0u32;
        let res = PackageFamilyNameFromFullName(PCWSTR(h_full_name.as_ptr()), &mut len, None);
        if res != ERROR_SUCCESS && res != ERROR_INSUFFICIENT_BUFFER {
            tracing::warn!(
                ?res,
                package_full_name,
                "context2: PackageFamilyNameFromFullName size query failed"
            );
            return None;
        }
        if len == 0 {
            return None;
        }

        let mut buffer_byte_count = 0u32;
        let mut count = 0u32;
        let _ = GetPackageApplicationIds(
            package_info_ref,
            &mut buffer_byte_count,
            None,
            Some(&mut count),
        );

        let mut aumid = None;
        if count > 0 {
            let mut buffer = vec![0u8; buffer_byte_count as usize];
            if let WIN32_ERROR(0) = GetPackageApplicationIds(
                package_info_ref,
                &mut buffer_byte_count,
                Some(buffer.as_mut_ptr()),
                Some(&mut count),
            ) {
                let ptr = buffer.as_ptr() as *const PWSTR;
                let first_app_id = ptr.read_unaligned();
                if let Some(family) = get_family_name(package_full_name) {
                    let app_id_str = first_app_id
                        .to_string()
                        .unwrap_or_else(|_| "App".to_string());
                    aumid = Some(format!("{family}!{app_id_str}"));
                }
            }
        }
        let _ = ClosePackageInfo(package_info_ref);

        let final_aumid = match aumid {
            Some(id) => id,
            None => {
                let family = get_family_name(package_full_name).unwrap_or_default();
                format!("{family}!App")
            }
        };
        if final_aumid.starts_with('!') || final_aumid.is_empty() {
            return None;
        }

        let h_aumid = HSTRING::from(&final_aumid);
        let HRESULT(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if e < 0 {
            tracing::warn!(hresult = e, "context2: CoInitializeEx failed");
        }

        let shell_item: IShellItem = SHCreateItemInKnownFolder(
            &FOLDERID_AppsFolder,
            KNOWN_FOLDER_FLAG(0),
            PCWSTR(h_aumid.as_ptr()),
        )
        .ok()?;
        let image_factory: IShellItemImageFactory = shell_item.cast().ok()?;
        let hbitmap = image_factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_RESIZETOFIT)
            .ok()?;
        let _guard = GdiObjectGuard(hbitmap);

        hbitmap_to_rgba(hbitmap)
    }
}

fn get_family_name(package_full_name: &str) -> Option<String> {
    unsafe {
        let h_full_name = HSTRING::from(package_full_name);
        let mut len = 0u32;
        let _ = PackageFamilyNameFromFullName(PCWSTR(h_full_name.as_ptr()), &mut len, None);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize];
        if PackageFamilyNameFromFullName(
            PCWSTR(h_full_name.as_ptr()),
            &mut len,
            Some(PWSTR(buf.as_mut_ptr())),
        )
        .is_ok()
        {
            return Some(
                String::from_utf16_lossy(&buf)
                    .trim_matches(char::from(0))
                    .to_string(),
            );
        }
        None
    }
}

unsafe fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Option<RgbaImage> {
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return None;
        }

        let mut bm = BITMAP::default();
        let res = GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );
        if res == 0 {
            ReleaseDC(None, hdc);
            return None;
        }

        let w = bm.bmWidth;
        let h = bm.bmHeight.abs();
        let mut buffer = vec![0u8; (w * h * 4) as usize];

        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };

        let scan_lines = GetDIBits(
            hdc,
            hbitmap,
            0,
            h as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        if scan_lines == 0 {
            return None;
        }

        let has_alpha = bgra_swap_and_detect_alpha(&mut buffer);
        if !has_alpha {
            for chunk in buffer.chunks_exact_mut(4) {
                chunk[3] = 255;
            }
        }
        Some(RgbaImage {
            pixels: buffer,
            width: w as u32,
            height: h as u32,
        })
    }
}

fn bgra_swap_and_detect_alpha(buffer: &mut [u8]) -> bool {
    let mut has_alpha = false;
    for chunk in buffer.chunks_exact_mut(4) {
        chunk.swap(0, 2);
        if chunk[3] > 0 {
            has_alpha = true;
        }
    }
    has_alpha
}

struct GdiObjectGuard(HBITMAP);
impl Drop for GdiObjectGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}
