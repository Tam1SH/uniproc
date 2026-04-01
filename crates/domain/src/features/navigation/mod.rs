mod actor;
mod settings;

use crate::features::navigation::actor::{
    NavigationActor, RequestPageSwitch, RequestTabAdd, RequestTabClose, RequestTabSwitch,
    SideBarWidthChanged,
};
use crate::features::navigation::settings::NavigationSettings;
use app_contracts::features::navigation::{
    page_ids, tab_ids, NavigationUiBindings, NavigationUiPort, PageDescriptor, TabDescriptor,
};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use app_core::SharedState;
use context::page_status::{
    PageId, PageStatusChanged, PageStatusRegistry, TabId, TabStatusChanged,
};

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
    P: NavigationUiPort + NavigationUiBindings + Clone + 'static,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = NavigationSettings::new(shared)?;
        let ui_port = (self.make_ui_port)(ui);

        let tabs = builtin_tabs();

        let default_page = settings.default_page().get();
        ui_port.set_navigation_tree(tabs.clone());
        ui_port.set_active_page(default_page.0, default_page.1);
        ui_port.set_side_bar_width(settings.side_bar_width().get());

        let actor = NavigationActor::new(
            ui_port.clone(),
            shared.get::<PageStatusRegistry>().unwrap(),
            tabs,
            &settings,
        );
        let addr = Addr::new(actor, ui.as_weak());

        let a = addr.clone();
        ui_port.on_request_page_switch(move |t_id, p_id| a.send(RequestPageSwitch(t_id, p_id)));

        let a = addr.clone();
        ui_port.on_request_tab_switch(move |t_id| a.send(RequestTabSwitch(t_id)));

        let a = addr.clone();
        ui_port.on_request_tab_close(move |t_id| a.send(RequestTabClose(t_id)));

        let a = addr.clone();
        ui_port.on_request_tab_add(move || a.send(RequestTabAdd));

        let a = addr.clone();
        ui_port.on_side_bar_width_changed(move |w| a.send(SideBarWidthChanged(w)));

        EventBus::subscribe::<_, PageStatusChanged, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, TabStatusChanged, _>(&ui.new_token(), addr.clone());

        Ok(())
    }
}

fn builtin_tabs() -> Vec<TabDescriptor> {
    vec![
        TabDescriptor {
            id: tab_ids::MAIN,
            title: "Dashboard".into(),
            pages: vec![
                PageDescriptor {
                    id: page_ids::PROCESSES,
                    text: "Processes".into(),
                    icon_key: "apps-list".into(),
                    ..Default::default()
                },
                PageDescriptor {
                    id: page_ids::SERVICES,
                    text: "Services".into(),
                    icon_key: "puzzle".into(),
                    ..Default::default()
                },
                PageDescriptor {
                    id: page_ids::DISK,
                    text: "Disk".into(),
                    icon_key: "disk".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        TabDescriptor {
            id: TabId(1),
            title: "Ubuntu".into(),
            pages: vec![PageDescriptor {
                id: PageId(0),
                text: "Processes".into(),
                icon_key: "proc".into(),
                ..Default::default()
            }],
            ..Default::default()
        },
    ]
}
