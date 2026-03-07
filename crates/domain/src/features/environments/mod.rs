use app_contracts::features::environments::{EnvironmentsUiBindings, EnvironmentsUiPort};
use app_core::SharedState;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

pub mod host;
pub mod wsl;

pub struct EnvironmentsFeature<F> {
    make_wsl_ui_port: F,
}

impl<F> EnvironmentsFeature<F> {
    pub fn new(make_wsl_ui_port: F) -> Self {
        Self { make_wsl_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for EnvironmentsFeature<F>
where
    TWindow: ComponentHandle + 'static,
    F: Fn(&TWindow) -> P + Clone + 'static,
    P: EnvironmentsUiPort + EnvironmentsUiBindings + Clone + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        host::HostFeature::new(self.make_wsl_ui_port.clone()).install(reactor, ui, shared)?;
        wsl::WslFeature::new(self.make_wsl_ui_port).install(reactor, ui, shared)?;
        Ok(())
    }
}

pub fn get_icon_for_env(name: &str) -> &'static str {
    let name_low = name.to_lowercase();

    match () {
        _ if name_low.contains("ubuntu") => "ubuntu",
        _ if name_low.contains("windows") => "windows-11",
        _ if name_low.contains("docker") => "docker",
        _ => "linux",
    }
}
