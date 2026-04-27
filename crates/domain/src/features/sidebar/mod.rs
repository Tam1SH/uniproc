pub mod actor;
pub mod settings;

use crate::features::sidebar::actor::{SideBarWidthChanged, SidebarActor};
use crate::features::sidebar::settings::SidebarSettings;
use app_contracts::features::sidebar::{SidebarBinder, UiSidebarBindings, UiSidebarPort};
use app_core::actor::addr::Addr;
use framework::app::Window;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

#[window_feature]
pub struct SidebarFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for SidebarFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiSidebarPort + UiSidebarBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let settings = SidebarSettings::new(ctx.shared)?;
        let ui_port = (self.make_port)(ctx.ui);

        ui_port.set_side_bar_width(settings.width().get());

        let actor = SidebarActor::new(ui_port.clone(), settings);
        let addr = Addr::new_managed(actor, ctx.ui.new_token(), &self.tracker);

        SidebarBinder::new(&addr, &ui_port).on_side_bar_width_changed(SideBarWidthChanged);

        Ok(())
    }
}
