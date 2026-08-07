use app_contracts2::features::services::ServiceRow;
use windows::Win32::System::Services::*;
use windows::core::PWSTR;

/// Temporary: scans services locally via `EnumServicesStatusExW`, ported
/// as-is from the old domain's `scanner/windows.rs`. This is a stand-in
/// until the windows-agent gains its own service enumeration and starts
/// reporting it as part of `WindowsReport` (see the
/// `services_scanner_belongs_to_agent` note) - at that point this whole
/// module goes away and `ServicesActor` switches to reacting to
/// `WindowsReportMessage` the same way `ProcessesActor` already does.
pub fn scan_services() -> anyhow::Result<Vec<ServiceRow>> {
    let mut results = Vec::new();

    unsafe {
        let sc_handle = OpenSCManagerW(
            None,
            None,
            SC_MANAGER_ENUMERATE_SERVICE | SC_MANAGER_CONNECT,
        )?;

        let mut bytes_needed = 0;
        let mut services_returned = 0;
        let mut resume_handle = 0;

        let _ = EnumServicesStatusExW(
            sc_handle,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            None,
        );

        let mut buffer = vec![0u8; bytes_needed as usize];

        EnumServicesStatusExW(
            sc_handle,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(buffer.as_mut_slice()),
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            None,
        )?;

        let services_ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
        let services = std::slice::from_raw_parts(services_ptr, services_returned as usize);

        for svc in services {
            let name = svc.lpServiceName.to_string().unwrap_or_default();
            let display_name = svc.lpDisplayName.to_string().unwrap_or_default();
            let pid = svc.ServiceStatusProcess.dwProcessId;

            let status = match svc.ServiceStatusProcess.dwCurrentState {
                SERVICE_RUNNING => "Running",
                SERVICE_STOPPED => "Stopped",
                SERVICE_PAUSED => "Paused",
                SERVICE_START_PENDING => "Starting...",
                SERVICE_STOP_PENDING => "Stopping...",
                _ => "Unknown",
            };

            let (group, description) =
                get_detailed_info(sc_handle, svc.lpServiceName).unwrap_or_default();

            results.push(ServiceRow {
                name,
                display_name,
                pid,
                status: status.to_string(),
                group,
                description,
            });
        }

        let _ = CloseServiceHandle(sc_handle);
    }

    Ok(results)
}

unsafe fn get_detailed_info(sc_handle: SC_HANDLE, svc_name: PWSTR) -> Option<(String, String)> {
    let Ok(h_service) = (unsafe { OpenServiceW(sc_handle, svc_name, SERVICE_QUERY_CONFIG) }) else {
        return None;
    };

    let mut group = String::new();
    let mut description = String::new();

    let mut dw_size = 0;
    let _ = unsafe { QueryServiceConfigW(h_service, None, 0, &mut dw_size) };
    let mut config_buf = vec![0u8; dw_size as usize];
    if unsafe {
        QueryServiceConfigW(
            h_service,
            Some(config_buf.as_mut_ptr() as *mut _),
            dw_size,
            &mut dw_size,
        )
    }
    .is_ok()
    {
        let config = config_buf.as_ptr() as *const QUERY_SERVICE_CONFIGW;
        group = unsafe { (*config).lpLoadOrderGroup.to_string() }.unwrap_or_default();
    }

    let _ = unsafe {
        QueryServiceConfig2W(h_service, SERVICE_CONFIG_DESCRIPTION, None, &mut dw_size)
    };
    let mut desc_buf = vec![0u8; dw_size as usize];
    if unsafe {
        QueryServiceConfig2W(
            h_service,
            SERVICE_CONFIG_DESCRIPTION,
            Some(&mut desc_buf),
            &mut dw_size,
        )
    }
    .is_ok()
    {
        let desc_ptr = desc_buf.as_ptr() as *const SERVICE_DESCRIPTIONW;
        if !unsafe { (*desc_ptr).lpDescription }.is_null() {
            description = unsafe { (*desc_ptr).lpDescription.to_string() }.unwrap_or_default();
        }
    }

    let _ = unsafe { CloseServiceHandle(h_service) };
    Some((group, description))
}
