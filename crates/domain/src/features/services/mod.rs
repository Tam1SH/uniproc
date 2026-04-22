use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::app::Window;
use std::collections::HashSet;
use std::sync::Arc;

use crate::features::services::application::actor::{
    OpenPropertiesWindow, OpenServices, ResizeCol, SelectedService, ServiceAction, ServiceActor,
    Sort, ViewportChanged,
};
use crate::features::services::application::snapshot_actor::ServiceSnapshotActor;
use crate::features::services::settings::ServiceSettings;
use crate::features::services::view::ServiceTable;
use app_contracts::features::agents::{ScanTick, WindowsActionResponse};
use app_contracts::features::navigation::{RouteActivated, TabContextKey};
use app_contracts::features::services::{
    ServicesWindowRegister, UiServicesBindings, UiServicesPort,
};
use app_contracts::features::windows_manager::OpenedWindow;
use app_core::feature::{FeatureContextState, WindowFeature, WindowFeatureInitContext};
use context::native_windows::slint_factory::SlintWindowRegistry;
use context::page_status::RouteStatusRegistry;
use macros::window_feature;

pub mod application;

mod scanner;
mod settings;
mod view;

#[window_feature]
pub struct ServicesFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for ServicesFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiServicesPort + UiServicesBindings + ServicesWindowRegister + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let settings = ServiceSettings::new(ctx.shared)?;
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();

        let reg = ctx.shared.get::<SlintWindowRegistry>().unwrap();

        let service_actor = ServiceActor {
            registry: reg.clone(),
            table: ServiceTable::new(settings.clone())?,
            ui_port: ui_port.clone(),
            route_status: ctx.shared.get::<RouteStatusRegistry>().unwrap(),
            is_active: true,
            active_context_key: TabContextKey::HOST,
            pending: HashSet::new(),
            ctx_state: FeatureContextState::new(ctx.window_id, "processes.list"),
        };

        let addr = Addr::new(service_actor, token.clone(), &self.tracker);

        let snapshot_actor = ServiceSnapshotActor {
            target: addr.clone(),
            is_active: true,
        };
        let snapshot_addr = Addr::new(snapshot_actor, token, &self.tracker);

        #[cfg(feature = "test-utils")]
        if let Some(registry) = ctx.shared.get::<app_core::actor::registry::ActorRegistry>() {
            registry.register(snapshot_addr.clone());
            registry.register(addr.clone());
        }

        let s_addr = snapshot_addr.clone();

        let loop_handle = ctx
            .reactor
            .add_dynamic_loop(settings.scan_interval_ms().as_signal(), move || {
                s_addr.send(ScanTick)
            });

        self.tracker.track_loop(loop_handle);

        bind_ui_events(addr.clone(), &ui_port, reg);

        EventBus::subscribe::<_, RouteActivated>(addr.clone(), &self.tracker);
        EventBus::subscribe::<_, RouteActivated>(snapshot_addr.clone(), &self.tracker);
        EventBus::subscribe::<_, WindowsActionResponse>(addr.clone(), &self.tracker);
        EventBus::subscribe::<_, OpenedWindow>(addr.clone(), &self.tracker);

        Ok(())
    }
}

fn bind_ui_events<P>(addr: Addr<ServiceActor<P>>, ui_port: &P, registry: Arc<SlintWindowRegistry>)
where
    P: UiServicesPort + UiServicesBindings + ServicesWindowRegister + Clone + 'static,
{
    let a = addr.clone();
    ui_port.on_service_action(move |name, action| {
        a.send(ServiceAction {
            name: name.to_string(),
            kind: action.into(),
        });
    });
    let a = addr.clone();
    ui_port.on_select_service(move |s_name, idx| a.send(SelectedService(s_name, idx as usize)));

    let a = addr.clone();
    ui_port.on_open_system_services(move || a.send(OpenServices));

    let a = addr.clone();
    ui_port.on_sort_by(move |field| a.send(Sort(field)));

    let a = addr.clone();
    ui_port.on_column_resized(move |id, width| a.send(ResizeCol { id, width }));

    let a = addr.clone();
    ui_port.on_rows_viewport_changed(move |start, count| {
        a.send(ViewportChanged {
            start: start as usize,
            count: count as usize,
        });
    });

    let a = addr.clone();
    ui_port.on_open_properties_window(move |service_entry| {
        a.send(OpenPropertiesWindow(service_entry));
    });

    ui_port.register(&registry);
}
