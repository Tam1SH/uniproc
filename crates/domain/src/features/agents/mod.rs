pub mod connection;

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod wsl;
        pub mod windows;
    } else {
        pub mod linux;
    }
}

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