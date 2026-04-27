use app_contracts::features::environments::{UiEnvironmentsBindings, UiEnvironmentsPort};
use framework::app::Window;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

pub mod host;
pub mod wsl;

#[window_feature]
pub struct EnvironmentsFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for EnvironmentsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + Clone + 'static,
    P: UiEnvironmentsPort + UiEnvironmentsBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        host::HostFeature::new(self.make_port.clone()).install(ctx)?;
        wsl::WslFeature::new(self.make_port.clone()).install(ctx)?;
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
