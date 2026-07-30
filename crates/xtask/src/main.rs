use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("run") => run(),
        _ => {
            eprintln!("Usage: cargo run -p xtask -- run");
            std::process::exit(1);
        }
    }
}

fn run() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let agent_dir = workspace_root.join("../uniproc-windows-agent");

    if !cfg!(target_os = "windows") {
        println!("Non-Windows host: starting desktop2 only.");
        return run_desktop(&workspace_root);
    }

    if agent_running() {
        println!("uniproc-windows-agent is already running.");
    } else {
        println!();
        println!("uniproc-windows-agent is not running.");
        println!("It must be started manually with administrator rights.");
        println!("In another terminal, run:");
        println!();
        println!("    cd {}", agent_dir.display());
        println!("    cargo run -- run");
        println!();
        println!("Press Enter here once the agent is running...");
        let _ = std::io::stdin().read_line(&mut String::new());

        if !agent_running() {
            anyhow::bail!("uniproc-windows-agent is still not running. Aborting.");
        }
    }

    run_desktop(&workspace_root)
}

fn run_desktop(workspace_root: &Path) -> anyhow::Result<()> {
    let mut desktop = Command::new("cargo")
        .args(["run", "-p", "desktop2"])
        .current_dir(workspace_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn desktop2: {e}"))?;

    let status = desktop.wait()?;
    if !status.success() {
        anyhow::bail!("desktop2 exited with {status}");
    }
    Ok(())
}

fn agent_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq uniproc-windows-agent.exe", "/NH"])
        .output();

    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.contains("uniproc-windows-agent.exe")
        }
        Err(_) => false,
    }
}

fn workspace_root() -> anyhow::Result<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| anyhow::anyhow!("CARGO_MANIFEST_DIR not set"))?;
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("cannot resolve workspace root"))
}
