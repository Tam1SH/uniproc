pub mod actor;
pub mod backend;
pub mod connection;
pub mod decode;
pub mod providers;
pub mod rpc;
pub mod settings;

use guinea::feature::{AppFeature, AppFeatureInitContext};
use guinea_macros::app_feature;
use tracing::info;

#[app_feature]
pub fn agents_feature(ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
    info!("Agents feature installed");
    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            providers::wsl::wsl_agent_feature(ctx)?;
            providers::windows::windows_agent_feature(ctx)?;
        } else {
            providers::linux::linux_agent_feature(ctx)?;
        }
    }

    Ok(())
}
