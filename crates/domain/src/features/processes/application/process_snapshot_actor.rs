use crate::processes_impl::application::actor::ProcessActor;
use crate::processes_impl::domain::snapshot::BridgeSnapshot;
use crate::processes_impl::scanner::base::ScanResult;
use crate::processes_impl::scanner::ctx::StatefulContext;
use crate::processes_impl::scanner::visitors::linux::WslScanResult;
use crate::processes_impl::scanner::visitors::windows::WindowsScanResult;
use app_contracts::features::agents::{RemoteScanResult, WindowsReportMessage};
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::{
    FieldDefDto, ProcessFieldDto, ProcessNodeDto, ProcessesUiPort,
};
use app_core::actor::addr::Addr;
use app_core::actor::traits::Message;
use app_core::actor::traits::{Context, Handler};
use app_core::messages;
use slint::{ComponentHandle, SharedString};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct ProcessSnapshotActor<P: ProcessesUiPort, TWindow: ComponentHandle + 'static> {
    pub snapshots: HashMap<&'static str, BridgeSnapshot>,
    pub contexts: HashMap<&'static str, Arc<StatefulContext>>,
    pub target: Addr<ProcessActor<P>, TWindow>,
    pub is_active: bool,
    pub scratch_processes: Arc<Mutex<Vec<ProcessNodeDto>>>,
    pub scratch_seen: HashSet<SharedString>,
}

messages! {
    ProcessSnapshotReady {
        column_defs: Vec<FieldDefDto>,
        processes: Arc<Mutex<Vec<ProcessNodeDto>>>,
        total_count: usize,
    }
}

impl<P: ProcessesUiPort, TWindow: ComponentHandle + 'static> ProcessSnapshotActor<P, TWindow> {
    fn context_for(&mut self, schema_id: &'static str) -> Arc<StatefulContext> {
        self.contexts
            .entry(schema_id)
            .or_insert_with(|| Arc::new(StatefulContext::new()))
            .clone()
    }

    fn rebuild_and_send(&mut self) {
        if self.snapshots.is_empty() {
            return;
        }

        let total_count: usize = self.snapshots.values().map(|s| s.processes.len()).sum();

        self.scratch_seen.clear();
        let mut column_defs: Vec<FieldDefDto> = Vec::new();

        {
            let mut processes = self.scratch_processes.lock().unwrap();
            processes.clear();

            for s in self.snapshots.values() {
                for def in &s.column_defs {
                    if self.scratch_seen.insert(def.id.clone()) {
                        column_defs.push(def.clone());
                    }
                }
                processes.extend_from_slice(&s.processes);
            }
        }

        self.target.send(ProcessSnapshotReady {
            column_defs,
            processes: self.scratch_processes.clone(),
            total_count,
        });
    }
}

impl<P, TWindow> Handler<TabChanged, TWindow> for ProcessSnapshotActor<P, TWindow>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: TabChanged, _ctx: &Context<Self, TWindow>) {
        self.is_active = msg.name == "Processes";
    }
}

impl<P, TWindow> Handler<RemoteScanResult, TWindow> for ProcessSnapshotActor<P, TWindow>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: RemoteScanResult, _ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }

        let ctx = self.context_for(msg.schema_id);
        let result = WslScanResult {
            processes: msg.processes,
            machine: msg.machine,
            ctx,
        };
        let snapshot = build_snapshot(&result);
        self.snapshots.insert(msg.schema_id, snapshot);
        self.rebuild_and_send();
    }
}

#[cfg(target_os = "windows")]
impl<P, TWindow> Handler<WindowsReportMessage, TWindow> for ProcessSnapshotActor<P, TWindow>
where
    P: ProcessesUiPort,
    TWindow: ComponentHandle + 'static,
{
    fn handle(&mut self, msg: WindowsReportMessage, _ctx: &Context<Self, TWindow>) {
        if !self.is_active {
            return;
        }

        let ctx = self.context_for("windows");
        let result = WindowsScanResult { report: msg.0, ctx };
        let snapshot = build_snapshot(&result);
        self.snapshots.insert("windows", snapshot);
        self.rebuild_and_send();
    }
}

pub fn build_snapshot(result: &dyn ScanResult) -> BridgeSnapshot {
    let mut column_defs: Vec<FieldDefDto> = vec![];

    result.visit_stats(&mut |mut field| {
        column_defs.push(FieldDefDto {
            id: field.id.clone(),
            label: field.label.clone(),
            stat_text: field.value.to_text(),
            stat_numeric: field.numeric,
            threshold: field.threshold,
            stat_detail: field.stat_detail,
            show_indicator: field.show_indicator,
        });
    });

    let ctx = result.context();
    let mut processes: Vec<ProcessNodeDto> = vec![];

    let mut fields: Vec<ProcessFieldDto> = Vec::new();
    result.visit_processes(&mut |proc| {
        fields.clear();
        proc.visit(&*ctx, &mut |mut field| {
            fields.push(ProcessFieldDto {
                id: field.id,
                text: field.value.to_text(),
                numeric: field.numeric,
                threshold: field.threshold,
            });
        });

        processes.push(ProcessNodeDto {
            pid: proc.pid(),
            name: proc.name(ctx),
            parent_pid: proc.parent_pid(),
            exe_path: proc.exe_path().map(|s| s.to_string().into()),
            fields: fields.clone(),
        });
    });

    BridgeSnapshot {
        column_defs,
        processes,
    }
}
