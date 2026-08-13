use windows::core::{Interface, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Storage::Packaging::Appx::{
    ClosePackageInfo, GetPackageApplicationIds, OpenPackageInfoByFullName,
    PackageFamilyNameFromFullName, _PACKAGE_INFO_REFERENCE,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::*;

use super::bitmap::{hbitmap_to_rgba, GdiObjectGuard, RgbaImage};

struct PackageInfoGuard(*mut _PACKAGE_INFO_REFERENCE);

impl Drop for PackageInfoGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = ClosePackageInfo(self.0);
            }
        }
    }
}

pub fn extract_appx_icon_rgba(package_full_name: &str, size: i32) -> Option<RgbaImage> {
    let family = family_name(package_full_name)?;
    let app_id = first_application_id(package_full_name)
        .unwrap_or_else(|| "App".to_string());
    let aumid = format!("{family}!{app_id}");

    unsafe {
        let h_aumid = HSTRING::from(&aumid);
        let HRESULT(e) = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if e < 0 {
            tracing::warn!(hresult = e, "context: CoInitializeEx failed");
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

fn first_application_id(package_full_name: &str) -> Option<String> {
    unsafe {
        let h_full_name = HSTRING::from(package_full_name);

        let mut package_info: *mut _PACKAGE_INFO_REFERENCE = std::ptr::null_mut();
        let res = OpenPackageInfoByFullName(
            PCWSTR(h_full_name.as_ptr()),
            None,
            &mut package_info,
        );
        if res != ERROR_SUCCESS || package_info.is_null() {
            tracing::debug!(
                ?res,
                package_full_name,
                "context: OpenPackageInfoByFullName failed"
            );
            return None;
        }
        let _guard = PackageInfoGuard(package_info);

        let mut byte_count = 0u32;
        let mut count = 0u32;
        let res = GetPackageApplicationIds(
            package_info,
            &mut byte_count,
            None,
            Some(&mut count),
        );
        if res != ERROR_INSUFFICIENT_BUFFER || byte_count == 0 {
            tracing::debug!(
                ?res,
                byte_count,
                package_full_name,
                "context: GetPackageApplicationIds size query failed"
            );
            return None;
        }

        let mut buffer = vec![0u64; byte_count.div_ceil(8) as usize];
        let res = GetPackageApplicationIds(
            package_info,
            &mut byte_count,
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut count),
        );
        if res != ERROR_SUCCESS || count == 0 {
            tracing::debug!(
                ?res,
                count,
                package_full_name,
                "context: GetPackageApplicationIds failed"
            );
            return None;
        }

        let first: PWSTR = (buffer.as_ptr() as *const PWSTR).read();
        if first.is_null() {
            return None;
        }
        first.to_string().ok()
    }
}

fn family_name(package_full_name: &str) -> Option<String> {
    unsafe {
        let h_full_name = HSTRING::from(package_full_name);

        let mut len = 0u32;
        let res = PackageFamilyNameFromFullName(PCWSTR(h_full_name.as_ptr()), &mut len, None);
        if res != ERROR_INSUFFICIENT_BUFFER || len == 0 {
            tracing::debug!(
                ?res,
                len,
                package_full_name,
                "context: PackageFamilyNameFromFullName size query failed"
            );
            return None;
        }

        let mut buf = vec![0u16; len as usize];
        let res = PackageFamilyNameFromFullName(
            PCWSTR(h_full_name.as_ptr()),
            &mut len,
            Some(PWSTR(buf.as_mut_ptr())),
        );
        if res != ERROR_SUCCESS {
            tracing::debug!(
                ?res,
                package_full_name,
                "context: PackageFamilyNameFromFullName failed"
            );
            return None;
        }

        buf.truncate(buf.iter().position(|&c| c == 0).unwrap_or(buf.len()));
        if buf.is_empty() {
            return None;
        }
        Some(String::from_utf16_lossy(&buf))
    }
}
