use crate::features::environments::wsl::domain::{RawDistroData, parse_wsl_output};

use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn fetch_distros_data() -> Vec<RawDistroData> {
    let Ok(out) = Command::new("wsl").args(["-l", "-v"]).output().await else {
        return Vec::new();
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut raw_data = Vec::new();

    for (name, is_running) in parse_wsl_output(&stdout) {
        let is_installed = check_agent_installed_async(&name).await;
        raw_data.push(RawDistroData {
            name,
            is_installed,
            is_running,
            latency_ms: -1,
        });
    }

    raw_data
}

pub async fn check_wsl_availability_async() -> anyhow::Result<bool> {
    Ok(Command::new("wsl")
        .arg("--status")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())?)
}

pub async fn check_agent_installed_async(distro: &str) -> bool {
    Command::new("wsl")
        .args(["-d", distro, "test", "-f", "/tmp/wsl_agent"])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

pub async fn inject_agent_async(distro: &str) -> anyhow::Result<()> {
    const AGENT_BIN: &[u8] = "".as_bytes();

    let mut child = Command::new("wsl")
        .args([
            "-d",
            distro,
            "sh",
            "-c",
            "cat > /tmp/wsl_agent && chmod +x /tmp/wsl_agent && /tmp/wsl_agent &",
        ])
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(AGENT_BIN).await?;
        stdin.flush().await?;
    }

    child.wait().await?;
    Ok(())
}
