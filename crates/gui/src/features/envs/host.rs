use super::get_icon_for_env;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::AppWindow;
use sysinfo::System;

pub struct HostFeature;

impl Feature for HostFeature {
    fn install(self, _reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let os_name = System::name().unwrap_or_else(|| "Windows".into());
        let os_icon = get_icon_for_env(&os_name);

        ui.set_host_name(os_name.clone().into());
        ui.set_host_icon(os_icon);

        ui.set_selected_env(os_name.into());

        Ok(())
    }
}
