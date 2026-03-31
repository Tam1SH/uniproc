use app_core::app::Window;
pub mod actor;
pub mod backend;
pub mod connection;
pub mod providers;
pub mod settings;

use crate::agents_impl::providers::{windows, wsl};
use app_core::SharedState;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;
use tracing::info;

pub struct AgentsFeature;

impl<TWindow: Window> Feature<TWindow> for AgentsFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        info!("Agents feature installed");
        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                wsl::WslAgentFeature.install(reactor, ui, shared)?;
                windows::WindowsAgentFeature.install(reactor, ui, shared)?;
            } else {
                linux::LinuxAgentFeature.install(reactor, ui, shared)?;
            }
        }

        Ok(())
    }
}
