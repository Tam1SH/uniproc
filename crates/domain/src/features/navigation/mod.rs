mod actor;
mod model;
mod settings;
mod state;

use crate::features::navigation::actor::{
    NavigationActor, RequestPageSwitch, RequestTabAdd, RequestTabClose, RequestTabSwitch,
    SideBarWidthChanged,
};
use crate::features::navigation::model::bootstrap_contexts;
use crate::features::navigation::settings::NavigationSettings;
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::environments::WslAgentRuntimeEvent;
use app_contracts::features::navigation::{NavigationUiBindings, UiNavigationPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use app_core::SharedState;
use context::page_status::{PageStatusChanged, PageStatusRegistry, TabStatusChanged};

pub struct NavigationFeature<F> {
    make_ui_port: F,
}

impl<F> NavigationFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for NavigationFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: UiNavigationPort + NavigationUiBindings + Clone + 'static,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = NavigationSettings::new(shared)?;
        let ui_port = (self.make_ui_port)(ui);

        let contexts = bootstrap_contexts();
        let actor = NavigationActor::new(
            ui_port.clone(),
            shared.get::<PageStatusRegistry>().unwrap(),
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
        let addr = Addr::new(actor, ui.as_weak());

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

        EventBus::subscribe::<_, PageStatusChanged, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, TabStatusChanged, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, RemoteScanResult, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, WslAgentRuntimeEvent, _>(&ui.new_token(), addr.clone());
        #[cfg(target_os = "windows")]
        EventBus::subscribe::<_, WindowsAgentRuntimeEvent, _>(&ui.new_token(), addr.clone());

        Ok(())
    }
}
