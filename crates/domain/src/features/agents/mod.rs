pub mod actor;
pub mod backend;
pub mod connection;
pub mod providers;
pub mod settings;

use crate::agents_impl::providers::{windows, wsl};
use framework::feature::{AppFeature, AppFeatureInitContext};
use tracing::info;

pub struct AgentsFeature;

impl AppFeature for AgentsFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
        info!("Agents feature installed");
        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                wsl::WslAgentFeature.install(ctx)?;
                windows::WindowsAgentFeature.install(ctx)?;
            } else {
                linux::LinuxAgentFeature.install(reactor, ui, shared)?;
            }
        }

        Ok(())
    }
}
