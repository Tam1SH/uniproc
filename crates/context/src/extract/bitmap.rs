use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO};

pub struct RgbaImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(super) unsafe fn hicon_to_rgba(hicon: HICON) -> Option<RgbaImage> {
    unsafe {
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).is_err() {
            return None;
        }
        let _color_guard = GdiObjectGuard(icon_info.hbmColor);
        let _mask_guard = GdiObjectGuard(icon_info.hbmMask);

        let hdc = GetDC(Option::from(HWND(std::ptr::null_mut())));
        if hdc.is_invalid() {
            return None;
        }

        let mut bm = BITMAP::default();
        let described = GetObjectW(
            icon_info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut _),
        );

        let w = bm.bmWidth;
        let h = bm.bmHeight.abs();
        if described == 0 || w <= 0 || h <= 0 {
            let _ = ReleaseDC(Option::from(HWND(std::ptr::null_mut())), hdc);
            return None;
        }
        let mut buffer = vec![0u8; w as usize * h as usize * 4];

        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0,
            ..std::mem::zeroed()
        };

        let scan_lines = GetDIBits(
            hdc,
            icon_info.hbmColor,
            0,
            h as u32,
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );
        if scan_lines == 0 {
            let _ = ReleaseDC(Option::from(HWND(std::ptr::null_mut())), hdc);
            return None;
        }

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
        let stride = ((w as usize).div_ceil(32)) * 4;
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
    let stride = ((w as usize).div_ceil(32)) * 4;
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

pub(super) unsafe fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Option<RgbaImage> {
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
        if w <= 0 || h <= 0 {
            ReleaseDC(None, hdc);
            return None;
        }
        let mut buffer = vec![0u8; w as usize * h as usize * 4];

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

pub(super) struct GdiObjectGuard(pub HBITMAP);
impl Drop for GdiObjectGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DeleteObject(self.0.into());
            }
        }
    }
}
