use crate::features::envs::get_icon_for_env;
use crate::features::envs::wsl::RawDistroData;
use crate::{AppWindow, WslDistro};

use slint::{ModelRc, VecModel};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

pub async fn fetch_distros_logic() -> Vec<RawDistroData> {
    let Ok(out) = Command::new("wsl").args(["-l", "-v"]).output().await else {
        return vec![];
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let distros_raw = parse_wsl_output(&stdout);

    let mut raw_data = Vec::new();
    for (name, is_running) in distros_raw {
        let is_installed = check_agent_installed_async(&name).await;
        raw_data.push(RawDistroData {
            name,
            is_installed,
            is_running,
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

pub async fn refresh_distros_async(ui_handle: slint::Weak<AppWindow>) {
    let output = Command::new("wsl").args(["-l", "-v"]).output().await.ok();

    if let Some(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let distros_raw = parse_wsl_output(&stdout);

        let mut raw_data = Vec::new();

        for (name, is_running) in distros_raw {
            let is_installed = check_agent_installed_async(&name).await;

            raw_data.push(RawDistroData {
                name,
                is_installed,
                is_running,
            });
        }

        slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui_handle.upgrade() {
                let distro_models: Vec<WslDistro> = raw_data
                    .into_iter()
                    .map(|rd| WslDistro {
                        name: rd.name.clone().into(),
                        is_installed: rd.is_installed,
                        is_running: rd.is_running,
                        icon: get_icon_for_env(&rd.name),
                    })
                    .collect();

                ui.set_wsl_distros(ModelRc::new(VecModel::from(distro_models)));
            }
        })
        .unwrap();
    }
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

pub fn parse_wsl_output(output: &str) -> Vec<(String, bool)> {
    let clean_output = output.replace('\0', "");
    clean_output
        .lines()
        .skip(1)
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();

            if parts[0] == "*" {
                if parts.len() >= 3 {
                    Some((
                        parts[1].to_string(),
                        parts[2].to_lowercase().contains("running"),
                    ))
                } else {
                    None
                }
            } else {
                if parts.len() >= 2 {
                    Some((
                        parts[0].to_string(),
                        parts[1].to_lowercase().contains("running"),
                    ))
                } else {
                    None
                }
            }
        })
        .collect()
}
