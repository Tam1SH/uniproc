use app_core::app::Window;
use app_core::app::{Feature, FromUiWeak};
use app_core::reactor::Reactor;

use crate::features::processes::application::actor::*;
use crate::features::processes::domain::table::ProcessTable;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::processes_impl::application::process_snapshot_actor::ProcessSnapshotActor;
use crate::processes_impl::settings::ProcessSettings;

#[cfg(target_os = "windows")]
use app_contracts::features::agents::WindowsReportMessage;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::{ProcessesUiBindings, ProcessesUiPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EventBus;
use app_core::SharedState;
use slint::ComponentHandle;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod application;
mod domain;
mod scanner;
mod services;
mod settings;

pub struct ProcessFeature<F> {
    pub show_icons: bool,
    make_ui_port: F,
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessFeatureBuilder {
    show_icons: bool,
}

impl<F> ProcessFeature<F> {
    pub fn new(show_icons: bool, make_ui_port: F) -> Self {
        Self {
            show_icons,
            make_ui_port,
        }
    }
}

impl ProcessFeature<()> {
    pub fn builder() -> ProcessFeatureBuilder {
        ProcessFeatureBuilder { show_icons: true }
    }
}

impl ProcessFeatureBuilder {
    pub fn show_icons(mut self, show_icons: bool) -> Self {
        self.show_icons = show_icons;
        self
    }

    pub fn with_ui_port<F>(self, make_ui_port: F) -> ProcessFeature<F> {
        ProcessFeature::new(self.show_icons, make_ui_port)
    }

    pub fn with_adapter<TWindow, TAdapter>(
        self,
    ) -> ProcessFeature<impl Fn(&TWindow) -> TAdapter + 'static>
    where
        TWindow: Window,
        TAdapter: FromUiWeak<TWindow> + ProcessesUiPort + ProcessesUiBindings + Clone + 'static,
    {
        ProcessFeature::new(self.show_icons, |ui: &TWindow| {
            TAdapter::from_ui_weak(ui.as_weak())
        })
    }
}

impl<TWindow, F, P> Feature<TWindow> for ProcessFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: ProcessesUiPort + ProcessesUiBindings + Clone + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = ProcessSettings::new(&shared)?;
        let ui_port = (self.make_ui_port)(ui);
        let scan_interval_ms = settings.scan_interval_ms();

        let process_actor = ProcessActor {
            table: ProcessTable::new(settings.clone())?,
            metadata: ProcessMetadataService,
            is_active: true,
            ui_port: ui_port.clone(),
            subs: vec![],
        };

        let addr = Addr::new(process_actor, ui.as_weak());

        let snapshot_actor = ProcessSnapshotActor {
            snapshots: HashMap::new(),
            contexts: HashMap::new(),
            target: addr.clone(),
            is_active: true,
            scratch_processes: Arc::new(Mutex::new(Vec::new())),
            scratch_seen: Default::default(),
        };
        let snapshot_addr = Addr::new(snapshot_actor, ui.as_weak());

        bind_ui_events(addr.clone(), &ui_port);

        EventBus::subscribe::<_, TabChanged, _>(&ui.new_token(), addr.clone());
        EventBus::subscribe::<_, TabChanged, _>(&ui.new_token(), snapshot_addr.clone());
        EventBus::subscribe::<_, RemoteScanResult, _>(&ui.new_token(), snapshot_addr.clone());
        #[cfg(target_os = "windows")]
        EventBus::subscribe::<_, WindowsReportMessage, _>(&ui.new_token(), snapshot_addr.clone());

        reactor.add_dynamic_loop(&scan_interval_ms, || EventBus::publish(ScanTick));

        Ok(())
    }
}

fn bind_ui_events<P, TWindow>(addr: Addr<ProcessActor<P>, TWindow>, ui_port: &P)
where
    TWindow: Window,
    P: ProcessesUiPort + ProcessesUiBindings + Clone + 'static,
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
}
