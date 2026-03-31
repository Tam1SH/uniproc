use super::get_icon_for_env;
use app_contracts::features::environments::EnvironmentsUiPort;
use app_core::SharedState;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use sysinfo::System;

pub struct HostFeature<F> {
    make_ui_port: F,
}

impl<F> HostFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for HostFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: EnvironmentsUiPort,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        _shared: &SharedState,
    ) -> anyhow::Result<()> {
        let os_name = System::name().unwrap_or_else(|| "Windows".into());
        let icon_key = get_icon_for_env(&os_name);
        let ui_port = (self.make_ui_port)(ui);
        ui_port.set_host_name(os_name.clone());
        ui_port.set_host_icon_by_key(icon_key);
        ui_port.set_selected_env(os_name);

        Ok(())
    }
}
