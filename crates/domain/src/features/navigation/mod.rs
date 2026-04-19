pub mod actor;
mod model;
mod settings;
mod state;

use crate::features::navigation::actor::{
    NavigationActor, RequestPageSwitch, RequestTabAdd, RequestTabClose, RequestTabSwitch,
    SideBarWidthChanged,
};
use crate::features::navigation::model::bootstrap_contexts;
use crate::features::navigation::settings::NavigationSettings;
use app_contracts::features::agents::RemoteScanResult;
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::environments::WslAgentRuntimeEvent;
use app_contracts::features::navigation::{UiNavigationBindings, UiNavigationPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Window;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};
use context::page_status::{PageStatusChanged, PageStatusRegistry, TabStatusChanged};
use macros::window_feature;

#[window_feature]
pub struct NavigationFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for NavigationFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiNavigationPort + UiNavigationBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let settings = NavigationSettings::new(ctx.shared)?;
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();

        let contexts = bootstrap_contexts();
        let actor = NavigationActor::new(
            ui_port.clone(),
            ctx.shared.get::<PageStatusRegistry>().unwrap(),
            contexts,
            &settings,
        );

        let tabs = actor.tabs().to_vec();
        let available_contexts = actor.available_contexts().to_vec();
        let default_page = settings.default_page().get();
        let initial_route = actor.resolve_initial_route(default_page).or_else(|| {
            tabs.first()
                .and_then(|tab| tab.pages.first().map(|page| (tab.id, page.id)))
        });

        let addr = Addr::new(actor, token, &self.tracker);

        #[cfg(feature = "test-utils")]
        if let Some(registry) = ctx.shared.get::<app_core::actor::registry::ActorRegistry>() {
            registry.register(addr.clone());
        }

        ui_port.set_navigation_tree(tabs.clone());
        ui_port.set_available_contexts(available_contexts);
        ui_port.set_side_bar_width(settings.side_bar_width().get());

        if let Some((tab_id, page_id)) = initial_route {
            addr.send(RequestPageSwitch(tab_id, page_id));
        }

        let a = addr.clone();
        ui_port.on_request_page_switch(move |t_id, p_id| a.send(RequestPageSwitch(t_id, p_id)));

        let a = addr.clone();
        ui_port.on_request_tab_switch(move |t_id| a.send(RequestTabSwitch(t_id)));

        let a = addr.clone();
        ui_port.on_request_tab_close(move |t_id| a.send(RequestTabClose(t_id)));

        let a = addr.clone();
        ui_port.on_request_tab_add(move |context_key| a.send(RequestTabAdd(context_key)));

        let a = addr.clone();
        ui_port.on_side_bar_width_changed(move |w| a.send(SideBarWidthChanged(w)));

        EventBus::subscribe_to(addr.clone(), &self.tracker).batch::<(
            PageStatusChanged,
            TabStatusChanged,
            RemoteScanResult,
            WslAgentRuntimeEvent,
        )>();

        #[cfg(target_os = "windows")]
        EventBus::subscribe::<_, WindowsAgentRuntimeEvent>(addr.clone(), &self.tracker);

        Ok(())
    }
}
