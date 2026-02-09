use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::AppWindow;
use slint::Image;
use tracing::instrument;

pub mod host;
pub mod wsl;

pub struct EnvironmentsFeature;

impl Feature for EnvironmentsFeature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        host::HostFeature.install(reactor, ui)?;
        wsl::WslFeature.install(reactor, ui)?;
        Ok(())
    }
}

#[tracing::instrument(target = "internal", skip_all)]
pub fn get_icon_for_env(name: &str) -> Image {
    let name_low = name.to_lowercase();

    let bytes: &[u8] = match () {
        _ if name_low.contains("ubuntu") => include_bytes!("../../../ui/assets/ubuntu.svg"),
        _ if name_low.contains("windows") || name_low.contains("microsoft") => {
            include_bytes!("../../../ui/assets/windows.svg")
        }
        _ if name_low.contains("docker") => include_bytes!("../../../ui/assets/docker.svg"),
        _ => include_bytes!("../../../ui/assets/linux.svg"),
    };

    tracing::debug!(target: "internal", "Icon data embedded for: {name}");

    Image::load_from_svg_data(bytes).unwrap_or_else(|e| {
        tracing::error!(target: "internal", "Failed to decode embedded icon for {name}: {e}");
        Image::default()
    })
}
