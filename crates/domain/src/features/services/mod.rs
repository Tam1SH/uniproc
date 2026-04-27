use crate::features::services::application::actor::{
    OpenPropertiesWindow, ResizeCol, SelectedService, ServiceAction, ServiceActor, Sort,
    ViewportChanged,
};
use crate::features::services::application::snapshot_actor::ServiceSnapshotActor;
use crate::features::services::settings::ServiceSettings;
use crate::features::services::view::ServiceTable;
use app_contracts::capabilities;
use app_contracts::features::agents::ScanTick;
use app_contracts::features::services::{
    ServicesBinder, ServicesWindowRegister, UiServicesBindings, UiServicesPort,
};
use app_core::actor::addr::Addr;
use context::page_status::RouteStatusRegistry;
use framework::app::Window;
use framework::feature::{FeatureContextState, WindowFeature, WindowFeatureInitContext};
use framework::native_windows::slint_factory::SlintWindowRegistry;
use macros::window_feature;
use std::borrow::Cow;
use std::collections::HashSet;

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
            active_context_key: Cow::Borrowed("host"),
            pending: HashSet::new(),
            ctx_state: FeatureContextState::new(ctx.window_id, capabilities::SERVICES),
        };

        let addr = Addr::new_managed(service_actor, token.clone(), &self.tracker);

        let snapshot_actor = ServiceSnapshotActor {
            target: addr.clone(),
            is_active: true,
        };
        let snapshot_addr = Addr::new_managed(snapshot_actor, token, &self.tracker);

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

        ServicesBinder::new(&addr, &ui_port)
            .on_service_action(|name, action| ServiceAction {
                name: name.to_string(),
                kind: action.into(),
            })
            .on_select_service(|s_name, idx| SelectedService(s_name, idx as usize))
            .on_sort_by(Sort)
            .on_column_resized(|id, width| ResizeCol { id, width })
            .on_rows_viewport_changed(|start, count| ViewportChanged {
                start: start as usize,
                count: count as usize,
            })
            .on_open_properties_window(OpenPropertiesWindow);

        ui_port.register(&reg);

        Ok(())
    }
}
