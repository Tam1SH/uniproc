use app_core::app::Window;
use app_core::feature::{WindowFeature, WindowFeatureInitContext};

use crate::features::processes::application::actor::*;
use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::ProcessSnapshotActor;
use crate::processes_impl::settings::ProcessSettings;

#[cfg(target_os = "windows")]
use app_contracts::features::agents::WindowsReportMessage;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::environments::WslAgentRuntimeEvent;
use app_contracts::features::navigation::{RouteActivated, TabContextKey};
use app_contracts::features::processes::{UiProcessesBindings, UiProcessesPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use context::page_status::RouteStatusRegistry;
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
            active_context_key: TabContextKey::HOST,
            is_grouped: false,
            ui_port: ui_port.clone(),
            has_snapshot_data: false,
            subs: vec![],
        };

        let addr = Addr::new(process_actor, token.clone(), &self.tracker);

        let snapshot_actor = ProcessSnapshotActor {
            snapshots: HashMap::new(),
            contexts: HashMap::new(),
            target: addr.clone(),
            is_active: true,
            scratch_processes: Arc::new(Mutex::new(Vec::new())),
            scratch_seen: Default::default(),
        };

        let snapshot_addr = Addr::new(snapshot_actor, token, &self.tracker);

        bind_ui_events(addr.clone(), &ui_port);

        let builder = EventBus::subscribe_to(addr.clone(), &self.tracker)
            .batch::<(RouteActivated, WslAgentRuntimeEvent)>();

        #[cfg(target_os = "windows")]
        builder.batch::<WindowsAgentRuntimeEvent>();

        let builder = EventBus::subscribe_to(snapshot_addr.clone(), &self.tracker)
            .batch::<(RouteActivated, RemoteScanResult)>();

        #[cfg(target_os = "windows")]
        builder.batch::<WindowsReportMessage>();

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

fn bind_ui_events<P>(addr: Addr<ProcessActor<P>>, ui_port: &P)
where
    P: UiProcessesPort + UiProcessesBindings + Clone + 'static,
{
    let a = addr.clone();
    ui_port.on_sort_by(move |field| a.send(Sort(field)));

    let a = addr.clone();
    ui_port.on_toggle_expand_group(move |group| a.send(ToggleExpand(group)));

    ui_port.on_terminate(addr.handler(TerminateSelected));

    let a = addr.clone();
    ui_port.on_select_process(move |pid, idx| {
        a.send(Select {
            pid: pid as u32,
            idx: idx as usize,
        })
    });

    let a = addr.clone();
    ui_port.on_column_resized(move |id, width| {
        a.send(ResizeColumn {
            id: id.into(),
            width,
        });
    });

    let a = addr.clone();
    ui_port.on_rows_viewport_changed(move |start, count| {
        a.send(ViewportChanged {
            start: start.max(0) as usize,
            count: count.max(0) as usize,
        })
    });

    let a = addr.clone();
    ui_port.on_group_clicked(move || a.send(GroupClicked));
}
