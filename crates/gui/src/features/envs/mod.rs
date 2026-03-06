use crate::AppWindow;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::shared::icons::Icons;
use app_core::SharedState;
use slint::Image;

pub mod host;
pub mod wsl;

pub struct EnvironmentsFeature;

impl Feature for EnvironmentsFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        host::HostFeature.install(reactor, ui, shared)?;
        wsl::WslFeature.install(reactor, ui, shared)?;
        Ok(())
    }
}

pub fn get_icon_for_env(name: &str) -> Image {
    let name_low = name.to_lowercase();

    let name = match () {
        _ if name_low.contains("ubuntu") => "ubuntu",
        _ if name_low.contains("windows") => "windows-11",
        _ if name_low.contains("docker") => "docker",
        _ => "linux",
    };

    Icons::get(name)
}
