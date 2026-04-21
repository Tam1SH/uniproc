use app_contracts::features::services::ServiceEntryDto;
use windows::Win32::System::Services::*;
use windows::core::PWSTR;

pub fn scan_services() -> anyhow::Result<Vec<ServiceEntryDto>> {
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

            let (group, desc) = get_detailed_info(sc_handle, svc.lpServiceName).unwrap_or_default();

            results.push(ServiceEntryDto {
                name,
                display_name,
                pid: pid as i32,
                status: status.into(),
                group,
                description: desc,
            });
        }

        let _ = CloseServiceHandle(sc_handle);
    }

    Ok(results)
}

unsafe fn get_detailed_info(sc_handle: SC_HANDLE, svc_name: PWSTR) -> Option<(String, String)> {
    let Ok(h_service) = OpenServiceW(sc_handle, svc_name, SERVICE_QUERY_CONFIG) else {
        return None;
    };

    let mut group = String::new();
    let mut desc = String::new();

    let mut dw_size = 0;
    let _ = QueryServiceConfigW(h_service, None, 0, &mut dw_size);
    let mut config_buf = vec![0u8; dw_size as usize];
    if QueryServiceConfigW(
        h_service,
        Some(config_buf.as_mut_ptr() as *mut _),
        dw_size,
        &mut dw_size,
    )
    .is_ok()
    {
        let config = config_buf.as_ptr() as *const QUERY_SERVICE_CONFIGW;
        group = (*config).lpLoadOrderGroup.to_string().unwrap_or_default();
    }

    let _ = QueryServiceConfig2W(h_service, SERVICE_CONFIG_DESCRIPTION, None, &mut dw_size);
    let mut desc_buf = vec![0u8; dw_size as usize];
    if QueryServiceConfig2W(
        h_service,
        SERVICE_CONFIG_DESCRIPTION,
        Some(&mut desc_buf),
        &mut dw_size,
    )
    .is_ok()
    {
        let description = desc_buf.as_ptr() as *const SERVICE_DESCRIPTIONW;
        if !(*description).lpDescription.is_null() {
            desc = (*description).lpDescription.to_string().unwrap_or_default();
        }
    }

    let _ = CloseServiceHandle(h_service);
    Some((group, desc))
}
