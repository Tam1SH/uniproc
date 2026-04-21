use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::ptr::null_mut;
use tracing::{debug, error, info, info_span, warn};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Storage::Packaging::Appx::{
    ClosePackageInfo, GetPackageApplicationIds, PackageFamilyNameFromFullName,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HRESULT, HSTRING, Interface, PCWSTR, PWSTR};

pub fn extract_appx_icon(package_full_name: &str, size: i32) -> Option<Image> {
    let span = info_span!("extract_appx_icon", package_full_name, size);
    let _enter = span.enter();

    unsafe {
        let h_full_name = HSTRING::from(package_full_name);
        let package_info_ref = null_mut();

        let mut len = 0u32;
        let res = PackageFamilyNameFromFullName(PCWSTR(h_full_name.as_ptr()), &mut len, None);

        if res != ERROR_SUCCESS && res != ERROR_INSUFFICIENT_BUFFER {
            warn!(error = ?res, "PackageFamilyNameFromFullName failed during size query");
            return None;
        }

        if len == 0 {
            warn!("Family Name length is 0");
            return None;
        }

        debug!("Package info reference opened successfully");

        let mut buffer_byte_count = 0u32;
        let mut count = 0u32;

        let _ = GetPackageApplicationIds(
            package_info_ref,
            &mut buffer_byte_count,
            None,
            Some(&mut count),
        );
        debug!(found_apps_count = count, "Queried application IDs count");

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
                    let generated_aumid = format!("{}!{}", family, app_id_str);
                    debug!(aumid = %generated_aumid, "Successfully constructed AUMID from Package IDs");
                    aumid = Some(generated_aumid);
                }
            } else {
                warn!("Failed to retrieve Application IDs data even though count > 0");
            }
        }

        let _ = ClosePackageInfo(package_info_ref);

        let final_aumid = match aumid {
            Some(id) => id,
            None => {
                let family = get_family_name(package_full_name).unwrap_or_default();
                let fallback = format!("{}!App", family);
                debug!(fallback_aumid = %fallback, "Using fallback AUMID construction");
                fallback
            }
        };

        if final_aumid.starts_with('!') || final_aumid.is_empty() {
            error!(aumid = %final_aumid, "Final AUMID is invalid or empty");
            return None;
        }

        let h_aumid = HSTRING::from(&final_aumid);

        let HRESULT(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if e < 0 {
            error!(error = e, "COM initialization failed");
        }

        let shell_item: IShellItem = match SHCreateItemInKnownFolder(
            &FOLDERID_AppsFolder,
            KNOWN_FOLDER_FLAG(0),
            PCWSTR(h_aumid.as_ptr()),
        ) {
            Ok(item) => {
                debug!("ShellItem created for AppsFolder");
                item
            }
            Err(e) => {
                warn!(error = ?e, aumid = %final_aumid, "SHCreateItemInKnownFolder failed - App might not be in AppsFolder");
                return None;
            }
        };

        let image_factory: IShellItemImageFactory = match shell_item.cast() {
            Ok(factory) => factory,
            Err(e) => {
                error!(error = ?e, "Failed to cast IShellItem to IShellItemImageFactory");
                return None;
            }
        };

        let hbitmap = match image_factory.GetImage(SIZE { cx: size, cy: size }, SIIGBF_RESIZETOFIT)
        {
            Ok(hb) => {
                debug!("HBITMAP successfully rendered by Shell");
                hb
            }
            Err(e) => {
                warn!(error = ?e, "IShellItemImageFactory::GetImage failed to render icon");
                return None;
            }
        };

        let _guard = ScopeGuard(hbitmap);

        match hbitmap_to_slint_image(hbitmap) {
            Some(img) => {
                info!(aumid = %final_aumid, "Successfully extracted AppX icon");
                Some(img)
            }
            None => {
                error!("hbitmap_to_slint_image conversion failed");
                None
            }
        }
    }
}

fn get_family_name(package_full_name: &str) -> Option<String> {
    use windows::Win32::Storage::Packaging::Appx::PackageFamilyNameFromFullName;
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
            Option::from(PWSTR(buf.as_mut_ptr())),
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

unsafe fn hbitmap_to_slint_image(hbitmap: HBITMAP) -> Option<Image> {
    use tracing::{debug, error, info, info_span};

    let span = info_span!("hbitmap_to_slint_image");
    let _enter = span.enter();

    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            error!("Failed to get Device Context (GetDC)");
            return None;
        }

        let mut bm = BITMAP::default();
        let res = GetObjectW(
            hbitmap.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );

        if res == 0 {
            error!("GetObjectW failed - HBITMAP might be invalid");
            ReleaseDC(None, hdc);
            return None;
        }

        let w = bm.bmWidth;
        let h = bm.bmHeight.abs();
        debug!(width = w, height = h, "Converting HBITMAP with dimensions");

        let mut buffer = vec![0u8; (w * h * 4) as usize];

        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // Top-down
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

        if scan_lines == 0 {
            error!("GetDIBits failed to extract pixels from HBITMAP");
            ReleaseDC(None, hdc);
            return None;
        }

        ReleaseDC(None, hdc);

        let mut has_alpha = false;

        for chunk in buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2);
            if chunk[3] > 0 {
                has_alpha = true;
            }
        }

        if !has_alpha {
            debug!("No alpha channel detected in HBITMAP, filling alpha with 255");
            for chunk in buffer.chunks_exact_mut(4) {
                chunk[3] = 255;
            }
        } else {
            debug!("Alpha channel detected and preserved");
        }

        let pixel_buffer =
            SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&buffer, w as u32, h as u32);
        info!("Successfully converted HBITMAP to Slint Image");
        Some(Image::from_rgba8(pixel_buffer))
    }
}

pub fn extract_icon_raw(path: &str) -> Option<Image> {
    unsafe {
        let mut shfi: SHFILEINFOW = std::mem::zeroed();
        let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();

        let result = SHGetFileInfoW(
            windows::core::PCWSTR(path_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        );

        if result == 0 || shfi.hIcon.is_invalid() || !has_own_icon(path) {
            return None;
        }

        let img = hicon_to_slint_image(shfi.hIcon);

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

    unsafe {
        let count = PrivateExtractIconsW(&buffer, 0, 0, 0, None, None, 0);
        count > 0
    }
}

unsafe fn hicon_to_slint_image(hicon: HICON) -> Option<Image> {
    unsafe {
        let mut icon_info = ICONINFO::default();

        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }

        let _color_guard = ScopeGuard(icon_info.hbmColor);
        let _mask_guard = ScopeGuard(icon_info.hbmMask);

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

        let _ = ReleaseDC(Option::from(HWND(std::ptr::null_mut())), hdc);

        let has_alpha = buffer.chunks_exact(4).any(|chunk| chunk[3] > 0);

        for chunk in buffer.chunks_exact_mut(4) {
            chunk.swap(0, 2);

            if !has_alpha {
                chunk[3] = 255;
            }
        }

        let pixel_buffer =
            SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&buffer, w as u32, h as u32);
        Some(Image::from_rgba8(pixel_buffer))
    }
}

struct ScopeGuard(HBITMAP);
impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}
