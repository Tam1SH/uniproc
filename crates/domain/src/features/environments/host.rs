use super::get_icon_for_env;
use app_contracts::features::environments::UiEnvironmentsPort;
use app_core::app::Window;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;
use sysinfo::System;

#[window_feature]
pub struct HostFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for HostFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiEnvironmentsPort,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let os_name = System::name().unwrap_or_else(|| "Windows".into());
        let icon_key = get_icon_for_env(&os_name);
        let ui_port = (self.make_port)(ctx.ui);
        ui_port.set_host_name(os_name.clone());
        ui_port.set_host_icon_by_key(icon_key);
        ui_port.set_selected_env(os_name);

        Ok(())
    }
}
