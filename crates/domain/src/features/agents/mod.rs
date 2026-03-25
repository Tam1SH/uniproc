pub mod connection;

#[cfg(target_os = "windows")]
pub mod wsl;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
pub mod linux;

use app_core::SharedState;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

pub struct AgentsFeature;

impl<TWindow: ComponentHandle + 'static> Feature<TWindow> for AgentsFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        #[cfg(target_os = "windows")]
        {
            wsl::WslAgentFeature.install(reactor, ui, shared)?;
            windows::WindowsAgentFeature.install(reactor, ui, shared)?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            linux::LinuxAgentFeature.install(reactor, ui, shared)?;
        }

        Ok(())
    }
}
