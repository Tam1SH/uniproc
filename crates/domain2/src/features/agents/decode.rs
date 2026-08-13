use app_contracts2::features::agents::{
    EnvironmentKind, LinuxDockerContainerInfo, LinuxEnvironmentInfo, LinuxMachineStats, LinuxProcessStats,
    LinuxReport, ProcessPriority, SignatureStatus, WindowsMachineStats, WindowsProcessStats, WindowsReport,
};
use uniproc_protocol::linux_capnp;
use uniproc_protocol::windows_capnp;

fn text(reader: capnp::text::Reader<'_>) -> capnp::Result<String> {
    Ok(reader.to_str()?.to_string())
}

pub fn windows_report(reader: windows_capnp::report::Reader<'_>) -> capnp::Result<WindowsReport> {
    let processes = reader
        .get_processes()?
        .iter()
        .map(windows_process)
        .collect::<capnp::Result<Vec<_>>>()?;

    Ok(WindowsReport {
        machine: windows_machine(reader.get_machine()?)?,
        processes,
    })
}

fn windows_machine(r: windows_capnp::machine_stats::Reader<'_>) -> capnp::Result<WindowsMachineStats> {
    Ok(WindowsMachineStats {
        total_physical_kb: r.get_total_physical_kb(),
        available_physical_kb: r.get_available_physical_kb(),
        used_physical_kb: r.get_used_physical_kb(),
        cpu_percent: r.get_cpu_percent(),
        cpu_max_mhz: r.get_cpu_max_mhz(),
        cpu_current_mhz: r.get_cpu_current_mhz(),
        disk_read_bytes: r.get_disk_read_bytes(),
        disk_write_bytes: r.get_disk_write_bytes(),
        disk_read_iops: r.get_disk_read_iops(),
        disk_write_iops: r.get_disk_write_iops(),
        net_rx_bytes: r.get_net_rx_bytes(),
        net_tx_bytes: r.get_net_tx_bytes(),
    })
}

fn windows_process(r: windows_capnp::process_stats::Reader<'_>) -> capnp::Result<WindowsProcessStats> {
    let cmdline = r
        .get_cmdline()?
        .iter()
        .map(|arg| text(arg?))
        .collect::<capnp::Result<Vec<_>>>()?;

    Ok(WindowsProcessStats {
        pid: r.get_pid(),
        parent_pid: r.get_parent_pid(),
        session_id: r.get_session_id(),
        name: text(r.get_name()?)?,
        cmdline,
        package_full_name: text(r.get_package_full_name()?)?,
        package_relative_app_id: text(r.get_package_relative_app_id()?)?,
        cpu_percent: r.get_cpu_percent(),
        working_set_kb: r.get_working_set_kb(),
        private_bytes_kb: r.get_private_bytes_kb(),
        peak_working_set_kb: r.get_peak_working_set_kb(),
        private_working_set_kb: r.get_private_working_set_kb(),
        disk_read_bytes: r.get_disk_read_bytes(),
        disk_write_bytes: r.get_disk_write_bytes(),
        disk_read_iops: r.get_disk_read_iops(),
        disk_write_iops: r.get_disk_write_iops(),
        net_rx_bytes: r.get_net_rx_bytes(),
        net_tx_bytes: r.get_net_tx_bytes(),
        is_service: r.get_is_service(),
        is_kernel_process: r.get_is_kernel_process(),
        is_windows_process: r.get_is_windows_process(),
        signature: r.get_signature().map(signature).unwrap_or_default(),
        image_path: text(r.get_image_path()?)?,
        display_name: text(r.get_display_name()?)?,
    })
}

fn signature(status: windows_capnp::SignatureStatus) -> SignatureStatus {
    match status {
        windows_capnp::SignatureStatus::Unknown => SignatureStatus::Unknown,
        windows_capnp::SignatureStatus::Unsigned => SignatureStatus::Unsigned,
        windows_capnp::SignatureStatus::Microsoft => SignatureStatus::Microsoft,
        windows_capnp::SignatureStatus::ThirdParty => SignatureStatus::ThirdParty,
    }
}

pub fn priority(priority: ProcessPriority) -> windows_capnp::ProcessPriority {
    match priority {
        ProcessPriority::Idle => windows_capnp::ProcessPriority::Idle,
        ProcessPriority::BelowNormal => windows_capnp::ProcessPriority::BelowNormal,
        ProcessPriority::Normal => windows_capnp::ProcessPriority::Normal,
        ProcessPriority::AboveNormal => windows_capnp::ProcessPriority::AboveNormal,
        ProcessPriority::High => windows_capnp::ProcessPriority::High,
        ProcessPriority::Realtime => windows_capnp::ProcessPriority::Realtime,
    }
}

pub fn linux_report(reader: linux_capnp::report::Reader<'_>) -> capnp::Result<LinuxReport> {
    let processes = reader
        .get_processes()?
        .iter()
        .map(linux_process)
        .collect::<capnp::Result<Vec<_>>>()?;
    let environments = reader
        .get_environments()?
        .iter()
        .map(linux_environment)
        .collect::<capnp::Result<Vec<_>>>()?;
    let docker_containers = reader
        .get_docker_containers()?
        .iter()
        .map(linux_docker_container)
        .collect::<capnp::Result<Vec<_>>>()?;

    Ok(LinuxReport {
        machine: linux_machine(reader.get_machine()?)?,
        processes,
        environments,
        docker_containers,
    })
}

fn linux_machine(r: linux_capnp::machine_stats::Reader<'_>) -> capnp::Result<LinuxMachineStats> {
    Ok(LinuxMachineStats {
        total_kb: r.get_total_kb(),
        free_kb: r.get_free_kb(),
        available_kb: r.get_available_kb(),
        used_kb: r.get_used_kb(),
        cached_kb: r.get_cached_kb(),
        busy_ns: r.get_busy_ns(),
        last_tsc: r.get_last_tsc(),
        vsock_rx_bytes: r.get_vsock_rx_bytes(),
        vsock_tx_bytes: r.get_vsock_tx_bytes(),
        p9_rx_bytes: r.get_p9_rx_bytes(),
        p9_tx_bytes: r.get_p9_tx_bytes(),
        tcp_tx_lo_bytes: r.get_tcp_tx_lo_bytes(),
        tcp_rx_lo_bytes: r.get_tcp_rx_lo_bytes(),
        tcp_tx_remote_bytes: r.get_tcp_tx_remote_bytes(),
        tcp_rx_remote_bytes: r.get_tcp_rx_remote_bytes(),
        udp_tx_lo_bytes: r.get_udp_tx_lo_bytes(),
        udp_rx_lo_bytes: r.get_udp_rx_lo_bytes(),
        udp_tx_remote_bytes: r.get_udp_tx_remote_bytes(),
        udp_rx_remote_bytes: r.get_udp_rx_remote_bytes(),
        uds_tx_bytes: r.get_uds_tx_bytes(),
        uds_rx_bytes: r.get_uds_rx_bytes(),
        disk_read_bytes: r.get_disk_read_bytes(),
        disk_write_bytes: r.get_disk_write_bytes(),
        disk_read_iops: r.get_disk_read_iops(),
        disk_write_iops: r.get_disk_write_iops(),
        pipe_read_bytes: r.get_pipe_read_bytes(),
        pipe_write_bytes: r.get_pipe_write_bytes(),
        sendfile_bytes: r.get_sendfile_bytes(),
        cpu_count: r.get_cpu_count(),
    })
}

fn linux_process(r: linux_capnp::process_stats::Reader<'_>) -> capnp::Result<LinuxProcessStats> {
    Ok(LinuxProcessStats {
        global_pid: r.get_global_pid(),
        local_pid: r.get_local_pid(),
        mnt_ns: r.get_mnt_ns(),
        pid_ns: r.get_pid_ns(),
        name: text(r.get_name()?)?,
        cpu_percent: r.get_cpu_percent(),
        rss_kb: r.get_rss_kb(),
        last_active_ns: r.get_last_active_ns(),
        vsock_rx_bytes: r.get_vsock_rx_bytes(),
        vsock_tx_bytes: r.get_vsock_tx_bytes(),
        p9_rx_bytes: r.get_p9_rx_bytes(),
        p9_tx_bytes: r.get_p9_tx_bytes(),
        tcp_tx_lo_bytes: r.get_tcp_tx_lo_bytes(),
        tcp_rx_lo_bytes: r.get_tcp_rx_lo_bytes(),
        tcp_tx_remote_bytes: r.get_tcp_tx_remote_bytes(),
        tcp_rx_remote_bytes: r.get_tcp_rx_remote_bytes(),
        udp_tx_lo_bytes: r.get_udp_tx_lo_bytes(),
        udp_rx_lo_bytes: r.get_udp_rx_lo_bytes(),
        udp_tx_remote_bytes: r.get_udp_tx_remote_bytes(),
        udp_rx_remote_bytes: r.get_udp_rx_remote_bytes(),
        uds_tx_bytes: r.get_uds_tx_bytes(),
        uds_rx_bytes: r.get_uds_rx_bytes(),
        disk_read_bytes: r.get_disk_read_bytes(),
        disk_write_bytes: r.get_disk_write_bytes(),
        disk_read_iops: r.get_disk_read_iops(),
        disk_write_iops: r.get_disk_write_iops(),
        pipe_read_bytes: r.get_pipe_read_bytes(),
        pipe_write_bytes: r.get_pipe_write_bytes(),
        sendfile_bytes: r.get_sendfile_bytes(),
    })
}

fn linux_environment(r: linux_capnp::environment_info::Reader<'_>) -> capnp::Result<LinuxEnvironmentInfo> {
    Ok(LinuxEnvironmentInfo {
        mnt_ns: r.get_mnt_ns(),
        pid_ns: r.get_pid_ns(),
        kind: r.get_kind().map(environment_kind).unwrap_or_default(),
        name: text(r.get_name()?)?,
    })
}

fn environment_kind(kind: linux_capnp::EnvironmentKind) -> EnvironmentKind {
    match kind {
        linux_capnp::EnvironmentKind::Unknown => EnvironmentKind::Unknown,
        linux_capnp::EnvironmentKind::CurrentDistro => EnvironmentKind::CurrentDistro,
        linux_capnp::EnvironmentKind::DockerContainer => EnvironmentKind::DockerContainer,
        linux_capnp::EnvironmentKind::UnknownExternalNamespace => EnvironmentKind::UnknownExternalNamespace,
    }
}

fn linux_docker_container(
    r: linux_capnp::docker_container_info::Reader<'_>,
) -> capnp::Result<LinuxDockerContainerInfo> {
    Ok(LinuxDockerContainerInfo {
        id: text(r.get_id()?)?,
        mnt_ns: r.get_mnt_ns(),
        pid_ns: r.get_pid_ns(),
        api_version: text(r.get_api_version()?)?,
        raw_json: text(r.get_raw_json()?)?,
    })
}
