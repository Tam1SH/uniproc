use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::{Feature, Window};
use app_core::reactor::Reactor;
use app_core::SharedState;

use crate::features::services::application::actor::{
    OpenServices, ServiceAction, ServiceActor, Sort, ViewportChanged,
};
use crate::features::services::application::snapshot_actor::ServiceSnapshotActor;
use crate::features::services::settings::ServiceSettings;
use crate::features::services::view::ServiceTable;
use app_contracts::features::agents::{ScanTick, WindowsActionResponse};
use app_contracts::features::navigation::{page_ids, PageActivated};
use app_contracts::features::services::{ServicesUiBindings, ServicesUiPort};
use context::page_status::PageStatusRegistry;

mod application;
mod scanner;
mod settings;
mod view;

pub struct ServicesFeature<F> {
    make_ui_port: F,
}

impl<F> ServicesFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for ServicesFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: ServicesUiPort + ServicesUiBindings + Clone + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = ServiceSettings::new(shared)?;
        let ui_port = (self.make_ui_port)(ui);

        let service_actor = ServiceActor {
            page_id: page_ids::SERVICES,
            table: ServiceTable::new(settings.clone())?,
            ui_port: ui_port.clone(),
            page_status: shared.get::<PageStatusRegistry>().unwrap(),
            is_active: true,
            pending: std::collections::HashSet::new(),
        };

        let addr = Addr::new(service_actor, ui.as_weak());

        let snapshot_actor = ServiceSnapshotActor {
            target: addr.clone(),
            page_id: page_ids::SERVICES,
            is_active: true,
        };
        let snapshot_addr = Addr::new(snapshot_actor, ui.as_weak());

        let s_addr = snapshot_addr.clone();
        reactor.add_dynamic_loop(settings.scan_interval_ms().as_signal(), move || {
            s_addr.send(ScanTick)
        });

        bind_ui_events(addr.clone(), &ui_port);

        EventBus::subscribe::<_, PageActivated, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, PageActivated, _>(&ui.new_token(), snapshot_addr.clone());
        EventBus::subscribe::<_, WindowsActionResponse, _>(&ui.new_token(), addr.clone());

        Ok(())
    }
}

fn bind_ui_events<P, TWindow>(addr: Addr<ServiceActor<P>, TWindow>, ui_port: &P)
where
    TWindow: Window,
    P: ServicesUiPort + ServicesUiBindings + Clone + 'static,
{
    let a = addr.clone();
    ui_port.on_service_action(move |name, action| {
        a.send(ServiceAction {
            name: name.to_string(),
            kind: action.into(),
        });
    });

    let a = addr.clone();
    ui_port.on_open_system_services(move || a.send(OpenServices));

    let a = addr.clone();
    ui_port.on_sort_by(move |field| a.send(Sort(field)));

    let a = addr.clone();
    ui_port.on_viewport_changed(move |start, count| {
        a.send(ViewportChanged { start, count });
    });
}
