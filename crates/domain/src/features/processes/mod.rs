use framework::app::Window;

use std::borrow::Cow;

use crate::features::processes::application::actor::*;
use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::ProcessSnapshotActor;
use crate::processes_impl::settings::ProcessSettings;

use app_contracts::features::agents::ScanTick;
use app_contracts::features::processes::{ProcessesBinder, UiProcessesBindings, UiProcessesPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use context::page_status::RouteStatusRegistry;
use framework::feature::{FeatureContextState, WindowFeature, WindowFeatureInitContext};
use macros::window_feature;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod application;
mod domain;
mod scanner;
mod services;
mod settings;

#[window_feature]
pub struct ProcessFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for ProcessFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: UiProcessesPort + UiProcessesBindings + Clone + 'static,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let settings = ProcessSettings::new(ctx.shared)?;
        let ui_port = (self.make_port)(ctx.ui);
        let token = ctx.ui.new_token();
        let scan_interval_ms = settings.scan_interval_ms();

        let process_actor = ProcessActor {
            table: ProcessTable::new(settings.clone())?,
            metadata: ProcessMetadataService,
            route_status: ctx.shared.get::<RouteStatusRegistry>().unwrap(),
            is_active: true,
            active_context_key: Cow::Borrowed("host"),
            is_grouped: false,
            ui_port: ui_port.clone(),
            has_snapshot_data: false,
            ctx: FeatureContextState::new(ctx.window_id, "processes.list"),
        };

        let addr = Addr::new_managed(process_actor, token.clone(), &self.tracker);

        let snapshot_actor = ProcessSnapshotActor {
            snapshots: HashMap::new(),
            contexts: HashMap::new(),
            target: addr.clone(),
            is_active: true,
            scratch_processes: Arc::new(Mutex::new(Vec::new())),
            scratch_seen: Default::default(),
        };

        let _ = Addr::new_managed(snapshot_actor, token, &self.tracker);

        ProcessesBinder::new(&addr, &ui_port)
            .on_sort_by(Sort)
            .on_toggle_expand_group(ToggleExpand)
            .on_terminate(TerminateSelected)
            .on_group_clicked(GroupClicked)
            .on_select_process(|pid, idx| Select {
                pid: pid as u32,
                idx: idx as usize,
            })
            .on_column_resized(|id, width| ResizeColumn {
                id: id.into(),
                width,
            })
            .on_rows_viewport_changed(|start, count| ViewportChanged {
                start: start.max(0) as usize,
                count: count.max(0) as usize,
            });

        let loop_handle = ctx
            .reactor
            .add_dynamic_loop(scan_interval_ms.as_signal(), || EventBus::publish(ScanTick));

        self.tracker.track_loop(loop_handle);

        //TODO: it broken + need translate
        ui_port.set_empty_state_visible(true);
        ui_port.set_empty_state_title("Waiting For Process Data".into());
        ui_port.set_empty_state_message(
            "The process list will appear after the agent connects and sends its first snapshot."
                .into(),
        );

        Ok(())
    }
}
