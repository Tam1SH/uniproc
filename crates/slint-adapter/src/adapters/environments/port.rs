use crate::adapters::environments::EnvironmentsUiAdapter;
use crate::{AppWindow, EnvironmentsFeatureGlobal, WslDistro};
use app_contracts::features::environments::{UiEnvironmentsPort, WslDistroDto};
use context::icons::Icons;
use macros::slint_port_adapter;
use slint::Image;
use slint::{ComponentHandle, ModelRc, VecModel};

fn get_icon_for_env(name: &str) -> Image {
    let name_low = name.to_lowercase();
    let icon_name = match () {
        _ if name_low.contains("ubuntu") => "ubuntu",
        _ if name_low.contains("windows") => "windows-11",
        _ if name_low.contains("docker") => "docker",
        _ => "linux",
    };

    Icons::get(icon_name)
}

#[slint_port_adapter(window = AppWindow)]
impl UiEnvironmentsPort for EnvironmentsUiAdapter {
    fn set_host_icon_by_key(&self, ui: &AppWindow, icon_key: &str) {
        ui.global::<EnvironmentsFeatureGlobal>()
            .set_host_icon(get_icon_for_env(icon_key))
    }

    fn set_wsl_distros(&self, ui: &AppWindow, distros: Vec<WslDistroDto>) {
        let model = distros
            .into_iter()
            .map(|distro| WslDistro {
                name: distro.name.clone().into(),
                is_installed: distro.is_installed,
                is_running: distro.is_running,
                icon: get_icon_for_env(&distro.name),
                latency_ms: distro.latency_ms,
            })
            .collect::<Vec<_>>();

        ui.global::<EnvironmentsFeatureGlobal>()
            .set_wsl_distros(ModelRc::new(VecModel::from(model)));
    }
}
