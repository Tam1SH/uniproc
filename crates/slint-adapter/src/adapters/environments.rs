use crate::{AppWindow, EnvironmentsFeatureGlobal, EnvsLoading, WslDistro};
use app_contracts::features::environments::{
    EnvironmentsUiBindings, EnvironmentsUiPort, WslDistroDto,
};
use slint::{ComponentHandle, Image, ModelRc, VecModel};

#[derive(Clone)]
pub struct EnvironmentsUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl EnvironmentsUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }

    fn with_ui<F>(&self, f: F)
    where
        F: FnOnce(&AppWindow),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui);
        }
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

impl EnvironmentsUiPort for EnvironmentsUiAdapter {
    fn set_host_name(&self, name: String) {
        self.with_ui(|ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_host_name(name.into())
        });
    }

    fn set_host_icon_by_key(&self, icon_key: &str) {
        self.with_ui(|ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_host_icon(get_icon_for_env(icon_key))
        });
    }

    fn set_selected_env(&self, name: String) {
        self.with_ui(|ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_selected_env(name.into())
        });
    }

    fn set_wsl_distros(&self, distros: Vec<WslDistroDto>) {
        self.with_ui(|ui| {
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
        });
    }

    fn set_has_wsl(&self, has_wsl: bool) {
        self.with_ui(|ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .set_has_wsl(has_wsl)
        });
    }

    fn set_wsl_is_loading(&self, loading: bool) {
        self.with_ui(|ui| ui.global::<EnvsLoading>().set_wsl_is_loading(loading));
    }

    fn set_wsl_distros_is_loading(&self, loading: bool) {
        self.with_ui(|ui| {
            ui.global::<EnvsLoading>()
                .set_wsl_distros_is_loading(loading)
        });
    }
}

impl EnvironmentsUiBindings for EnvironmentsUiAdapter {
    fn on_install_agent<F>(&self, handler: F)
    where
        F: Fn(String) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<EnvironmentsFeatureGlobal>()
                .on_install_agent(move |distro| handler(distro.to_string()));
        });
    }
}
