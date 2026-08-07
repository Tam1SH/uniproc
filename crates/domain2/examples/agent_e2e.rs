//! End-to-end probe of the agent client stack against *real* running agents.
//!
//! Not a `#[test]` on purpose: it needs `uniproc-windows-agent` running with
//! administrator rights (and, for the `--wsl` half, a WSL VM with the Linux
//! agent listening on vsock). That is a property of one developer machine, not
//! something CI or a fresh checkout can satisfy, so it stays a thing you run
//! deliberately:
//!
//! ```text
//! cargo run -p xtask -- agent-check          # checks the agent is up first
//! cargo run -p domain2 --example agent_e2e   # or drive it directly
//! cargo run -p domain2 --example agent_e2e -- --wsl
//! ```
//!
//! What it actually exercises is the part that is ours and unproven: the
//! `RpcHandle` thread bridge (tokio caller -> compio/capnp thread), the capnp
//! -> owned-DTO decoders, and the `AgentBackend` impls. It deliberately does
//! *not* call `perform_scan`, which publishes onto `GlobalEventBus` and needs
//! the actor runtime to be up; the only thing that adds over what is checked
//! here is a match arm and a publish.
//!
//! Read-only: it issues no process or service commands.

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
    use app_contracts2::features::agents::{SignatureStatus, WindowsProcessStats, WindowsReport};
    use domain2::features::agents::backend::AgentBackend;
    use domain2::features::agents::providers::windows::{
        WindowsBackend, WindowsReply, WindowsRequest, WindowsRpc,
    };
    use domain2::features::agents::rpc::RpcHandle;
    use std::time::{Duration, Instant};

    const CONNECT_TIMEOUT_SECS: u64 = 5;

    pub async fn probe() -> anyhow::Result<()> {
        println!("== windows agent ==");

        let started = Instant::now();
        let handle = RpcHandle::<WindowsRpc>::connect(CONNECT_TIMEOUT_SECS)
            .await
            .context("connect failed - is uniproc-windows-agent running (as admin)?")?;
        println!("connect: ok ({} ms)", started.elapsed().as_millis());

        // The backend trait is what the actor actually calls, so probe it
        // rather than only the layer underneath it.
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

        // Sanity, not decoration: an all-zero machine block or an empty
        // process list means the decode silently produced defaults.
        assert!(m.total_physical_kb > 0, "total physical memory decoded as 0");
        assert!(!report.processes.is_empty(), "no processes in the report");

        let enriched: Vec<&WindowsProcessStats> = report
            .processes
            .iter()
            .filter(|p| p.is_service || p.has_visible_window || p.is_kernel_process)
            .collect();

        println!(
            "  enrichment: {} services, {} with a window, {} kernel, {} signed by microsoft",
            report.processes.iter().filter(|p| p.is_service).count(),
            report.processes.iter().filter(|p| p.has_visible_window).count(),
            report.processes.iter().filter(|p| p.is_kernel_process).count(),
            report
                .processes
                .iter()
                .filter(|p| p.signature == SignatureStatus::Microsoft)
                .count(),
        );

        for p in enriched.iter().take(6) {
            println!(
                "    pid={:<6} {:<26} svc={} win={} krn={} sig={:?} {}",
                p.pid,
                truncate(&p.name, 26),
                p.is_service,
                p.has_visible_window,
                p.is_kernel_process,
                p.signature,
                truncate(&p.image_path, 48),
            );
        }
    }

    /// Dispatches are spawned per request precisely so a slow one cannot hold
    /// up the rest; this is what proves that. A `getReport` on a busy machine
    /// takes long enough to notice, so pings issued alongside it must keep
    /// answering promptly instead of queueing behind it.
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

    /// Dropping the last handle is the only disconnect path, so it had better
    /// actually close the session rather than leaking the thread.
    async fn teardown_probe(handle: RpcHandle<WindowsRpc>) -> anyhow::Result<()> {
        let orphan = handle.clone();
        drop(handle);

        // A surviving clone must still work - teardown is on the *last* drop.
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
    use domain2::features::agents::backend::AgentBackend;
    use domain2::features::agents::providers::wsl::{WslBackend, WslReply, WslRequest, WslRpc};
    use domain2::features::agents::rpc::RpcHandle;
    use std::time::Instant;

    /// Far longer than the Windows side: this launches the agent itself, and it
    /// has to bring eBPF up before it starts listening.
    const CONNECT_TIMEOUT_SECS: u64 = 40;

    pub async fn probe() -> anyhow::Result<()> {
        println!("== wsl agent ==");

        // Normally published by `wsl_agent_feature`; the probe does not boot
        // the feature graph, so it has to say where the agent lives itself.
        // Override with WSL_DISTRO / WSL_AGENT_PATH when testing a build that
        // is not the installed one.
        let distro = std::env::var("WSL_DISTRO").unwrap_or_else(|_| "Ubuntu".to_string());
        let agent_path = std::env::var("WSL_AGENT_PATH")
            .unwrap_or_else(|_| "/usr/local/bin/uniproc-agent".to_string());
        println!("launching {agent_path} in {distro}");
        domain2::features::agents::providers::wsl::set_launch_config(distro, agent_path);

        let started = Instant::now();
        let handle = RpcHandle::<WslRpc>::connect(CONNECT_TIMEOUT_SECS)
            .await
            .context("connect failed - is the Linux agent running inside WSL?")?;
        println!("connect: ok ({} ms)", started.elapsed().as_millis());

        let latency = WslBackend::ping(&handle).await.context("ping failed")?;
        println!("ping via AgentBackend: {latency} ms");

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

                for p in report.processes.iter().take(5) {
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
