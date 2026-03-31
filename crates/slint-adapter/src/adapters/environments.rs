use crate::{AppWindow, EnvironmentsFeatureGlobal, EnvsLoading, WslDistro};
use app_contracts::features::environments::{
    EnvironmentsUiBindings, EnvironmentsUiPort, WslDistroDto,
};
use macros::ui_adapter;
use slint::{ComponentHandle, Image, ModelRc, VecModel};

#[derive(Clone)]
pub struct EnvironmentsUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl EnvironmentsUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}

fn get_icon_for_env(name: &str) -> Image {
    let name_low = name.to_lowercase();
    let icon_name = match () {
        _ if name_low.contains("ubuntu") => "ubuntu",
        _ if name_low.contains("windows") => "windows-11",
        _ if name_low.contains("docker") => "docker",
        _ => "linux",
    };

    let bytes: &[u8] = match icon_name {
        "ubuntu" => include_bytes!("../../ui/assets/ubuntu.svg"),
        "windows-11" => include_bytes!("../../ui/assets/windows-11.svg"),
        "docker" => include_bytes!("../../ui/assets/docker.svg"),
        "linux" => include_bytes!("../../ui/assets/linux.svg"),
        _ => return Image::default(),
    };

    Image::load_from_svg_data(bytes).unwrap_or_default()
}

#[ui_adapter]
impl EnvironmentsUiPort for EnvironmentsUiAdapter {
    fn set_host_name(&self, ui: &AppWindow, name: String) {
        ui.global::<EnvironmentsFeatureGlobal>()
            .set_host_name(name.into())
    }

    fn set_host_icon_by_key(&self, ui: &AppWindow, icon_key: &str) {
        ui.global::<EnvironmentsFeatureGlobal>()
            .set_host_icon(get_icon_for_env(icon_key))
    }

    fn set_selected_env(&self, ui: &AppWindow, name: String) {
        ui.global::<EnvironmentsFeatureGlobal>()
            .set_selected_env(name.into())
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

    fn set_has_wsl(&self, ui: &AppWindow, has_wsl: bool) {
        ui.global::<EnvironmentsFeatureGlobal>().set_has_wsl(has_wsl)
    }

    fn set_wsl_is_loading(&self, ui: &AppWindow, loading: bool) {
        ui.global::<EnvsLoading>().set_wsl_is_loading(loading);
    }

    fn set_wsl_distros_is_loading(&self, ui: &AppWindow, loading: bool) {
        ui.global::<EnvsLoading>()
            .set_wsl_distros_is_loading(loading)
    }
}

#[ui_adapter]
impl EnvironmentsUiBindings for EnvironmentsUiAdapter {
    fn on_install_agent<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<EnvironmentsFeatureGlobal>()
            .on_install_agent(move |distro| handler(distro.to_string()));
    }
}
