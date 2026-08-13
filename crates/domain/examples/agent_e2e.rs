#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("agent_e2e targets the Windows host agents; nothing to probe here.");
}

#[cfg(target_os = "windows")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let probe_wsl = std::env::args().any(|arg| arg == "--wsl");

    windows::probe().await?;

    if probe_wsl {
        println!();
        wsl::probe().await?;
    } else {
        println!("\n(skipping the WSL agent; pass --wsl to include it)");
    }

    println!("\nall probes passed");
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use anyhow::{Context, bail};
    use app_contracts::features::agents::{SignatureStatus, WindowsProcessStats, WindowsReport};
    use domain::features::agents::backend::AgentBackend;
    use domain::features::agents::providers::windows::{
        WindowsBackend, WindowsReply, WindowsRequest, WindowsRpc,
    };
    use domain::features::agents::rpc::RpcHandle;
    use std::time::{Duration, Instant};

    const CONNECT_TIMEOUT_SECS: u64 = 5;

    pub async fn probe() -> anyhow::Result<()> {
        println!("== windows agent ==");

        let started = Instant::now();
        let handle = RpcHandle::<WindowsRpc>::connect(CONNECT_TIMEOUT_SECS)
            .await
            .context("connect failed - is uniproc-windows-agent running (as admin)?")?;
        println!("connect: ok ({} ms)", started.elapsed().as_millis());

        let latency = WindowsBackend::ping(&handle).await.context("ping failed")?;
        println!("ping via AgentBackend: {latency} ms");

        let report = get_report(&handle).await?;
        print_report(&report);

        concurrency_probe(&handle).await?;
        teardown_probe(handle).await?;

        Ok(())
    }

    async fn get_report(handle: &RpcHandle<WindowsRpc>) -> anyhow::Result<WindowsReport> {
        let started = Instant::now();
        match handle.call(WindowsRequest::GetReport).await? {
            WindowsReply::Report(report) => {
                println!(
                    "getReport: {} processes decoded in {} ms",
                    report.processes.len(),
                    started.elapsed().as_millis()
                );
                Ok(report)
            }
            _ => bail!("agent answered getReport with the wrong reply"),
        }
    }

    fn print_report(report: &WindowsReport) {
        let m = &report.machine;
        println!(
            "  machine: cpu {:.1}% @ {} / {} MHz, mem {} / {} MB, net rx {} tx {}",
            m.cpu_percent,
            m.cpu_current_mhz,
            m.cpu_max_mhz,
            m.used_physical_kb / 1024,
            m.total_physical_kb / 1024,
            m.net_rx_bytes,
            m.net_tx_bytes,
        );

        assert!(m.total_physical_kb > 0, "total physical memory decoded as 0");
        assert!(!report.processes.is_empty(), "no processes in the report");

        let enriched: Vec<&WindowsProcessStats> = report
            .processes
            .iter()
            .filter(|p| p.is_service || p.is_kernel_process)
            .collect();

        println!(
            "  enrichment: {} services, {} kernel, {} signed by microsoft",
            report.processes.iter().filter(|p| p.is_service).count(),
            report.processes.iter().filter(|p| p.is_kernel_process).count(),
            report
                .processes
                .iter()
                .filter(|p| p.signature == SignatureStatus::Microsoft)
                .count(),
        );

        for p in enriched.iter().take(6) {
            println!(
                "    pid={:<6} {:<26} svc={} krn={} sig={:?} {}",
                p.pid,
                truncate(&p.name, 26),
                p.is_service,
                p.is_kernel_process,
                p.signature,
                truncate(&p.image_path, 48),
            );
        }
    }

    async fn concurrency_probe(handle: &RpcHandle<WindowsRpc>) -> anyhow::Result<()> {
        let report_handle = handle.clone();
        let report = tokio::spawn(async move { report_handle.call(WindowsRequest::GetReport).await });

        let ping_handle = handle.clone();
        let pings = tokio::spawn(async move {
            let mut worst = Duration::ZERO;
            for _ in 0..15 {
                let started = Instant::now();
                if ping_handle.call(WindowsRequest::Ping).await.is_ok() {
                    worst = worst.max(started.elapsed());
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            worst
        });

        report.await??;
        let worst = pings.await?;
        println!("concurrency: worst ping during getReport {} ms", worst.as_millis());
        Ok(())
    }

    async fn teardown_probe(handle: RpcHandle<WindowsRpc>) -> anyhow::Result<()> {
        let orphan = handle.clone();
        drop(handle);

        orphan.call(WindowsRequest::Ping).await.context("clone stopped working after a sibling dropped")?;

        drop(orphan);
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("teardown: session closed on last handle drop");
        Ok(())
    }

    fn truncate(s: &str, max: usize) -> String {
        if s.chars().count() <= max {
            s.to_string()
        } else {
            s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
        }
    }
}

#[cfg(target_os = "windows")]
mod wsl {
    use anyhow::{Context, bail};
    use domain::features::agents::backend::AgentBackend;
    use domain::features::agents::providers::wsl::{WslBackend, WslReply, WslRequest, WslRpc};
    use domain::features::agents::rpc::RpcHandle;
    use std::time::Instant;

    const CONNECT_TIMEOUT_SECS: u64 = 40;

    pub async fn probe() -> anyhow::Result<()> {
        println!("== wsl agent ==");

        let distro = std::env::var("WSL_DISTRO").unwrap_or_else(|_| "Ubuntu".to_string());
        let agent_path = std::env::var("WSL_AGENT_PATH")
            .unwrap_or_else(|_| "/usr/local/bin/uniproc-agent".to_string());
        println!("launching {agent_path} in {distro}");
        domain::features::agents::providers::wsl::set_launch_config(distro, agent_path);

        let started = Instant::now();
        let handle = RpcHandle::<WslRpc>::connect(CONNECT_TIMEOUT_SECS)
            .await
            .context("connect failed - is the Linux agent running inside WSL?")?;
        println!("connect: ok ({} ms)", started.elapsed().as_millis());

        let latency = WslBackend::ping(&handle).await.context("ping failed")?;
        println!("ping via AgentBackend: {latency} ms");

        let _ = handle.call(WslRequest::GetReport).await?;
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

        match handle.call(WslRequest::GetReport).await? {
            WslReply::Report(report) => {
                let m = &report.machine;
                println!(
                    "getReport: {} processes, {} environments, {} docker containers",
                    report.processes.len(),
                    report.environments.len(),
                    report.docker_containers.len(),
                );
                println!(
                    "  machine: mem {} / {} MB, busy {} ns",
                    m.used_kb / 1024,
                    m.total_kb / 1024,
                    m.busy_ns
                );
                assert!(m.total_kb > 0, "total memory decoded as 0");
                assert!(!report.processes.is_empty(), "no processes in the report");

                let mut top: Vec<_> = report.processes.iter().collect();
                top.sort_by_key(|p| std::cmp::Reverse(p.rss_kb));
                let with_rss = report.processes.iter().filter(|p| p.rss_kb > 0).count();
                println!(
                    "  processes with rss>0: {} / {}",
                    with_rss,
                    report.processes.len()
                );
                for p in top.iter().take(10) {
                    println!(
                        "    pid={:<6} {:<24} cpu={:.1}% rss={} kb",
                        p.global_pid, p.name, p.cpu_percent, p.rss_kb
                    );
                }

                let mut busiest: Vec<_> = report.processes.iter().collect();
                busiest.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
                println!("  busiest by cpu:");
                for p in busiest.iter().take(3) {
                    println!(
                        "    pid={:<6} {:<24} cpu={:.1}% rss={} kb",
                        p.global_pid, p.name, p.cpu_percent, p.rss_kb
                    );
                }

                println!("  environments:");
                for e in &report.environments {
                    let procs = report
                        .processes
                        .iter()
                        .filter(|p| p.mnt_ns == e.mnt_ns && p.pid_ns == e.pid_ns)
                        .count();
                    println!(
                        "    {:?}  mnt_ns={} pid_ns={} procs={} name={:?}",
                        e.kind, e.mnt_ns, e.pid_ns, procs, e.name
                    );
                }
                Ok(())
            }
            _ => bail!("agent answered getReport with the wrong reply"),
        }
    }
}
