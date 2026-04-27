pub mod actor;
mod model;
mod state;

use crate::features::tabs::actor::{RequestTabAdd, RequestTabClose, RequestTabSwitch, TabsActor};
use crate::features::tabs::model::bootstrap_contexts;
use app_contracts::features::tabs::{TabsBinder, UiTabsBindings, UiTabsPort};
use app_core::actor::addr::Addr;
use framework::app::Window;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
use framework::navigation::RouteRegistry;
use macros::window_feature;

#[window_feature]
pub struct TabsFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for TabsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + Clone + 'static,
    P: UiTabsPort + UiTabsBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();

        let routes = ctx
            .shared
            .get::<RouteRegistry>()
            .expect("RouteRegistry must be installed before TabsFeature");

        let contexts = bootstrap_contexts();
        let actor = TabsActor::new(ui_port.clone(), contexts, routes);

        let tabs = actor.tabs().to_vec();
        let available_contexts = actor.available_contexts().to_vec();
        let active_context_key = actor.active_context_key().cloned();

        let addr = Addr::new_managed(actor, token, &self.tracker);

        #[cfg(feature = "test-utils")]
        if let Some(registry) = ctx.shared.get::<app_core::actor::registry::ActorRegistry>() {
            registry.register(addr.clone());
        }

        ui_port.set_tabs(tabs);
        ui_port.set_available_contexts(available_contexts);
        if let Some(active_context_key) = active_context_key {
            ui_port.set_active_context(active_context_key);
        }

        TabsBinder::new(&addr, &ui_port)
            .on_request_tab_switch(RequestTabSwitch)
            .on_request_tab_close(RequestTabClose)
            .on_request_tab_add(RequestTabAdd);

        Ok(())
    }
}
