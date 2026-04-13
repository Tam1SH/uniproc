use crate::processes_impl::application::actor::ProcessActor;
use crate::processes_impl::domain::snapshot::BridgeSnapshot;
use crate::processes_impl::scanner::base::ScanResult;
use crate::processes_impl::scanner::ctx::StatefulContext;
use crate::processes_impl::scanner::visitors::linux::WslScanResult;
use crate::processes_impl::scanner::visitors::windows::WindowsScanResult;
use app_contracts::features::agents::{RemoteScanResult, WindowsReportMessage};
use app_contracts::features::navigation::PageActivated;
use app_contracts::features::processes::{
    FieldDefDto, ProcessFieldDto, ProcessNodeDto, UiProcessesPort,
};
use app_core::actor::addr::Addr;
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;
use app_core::{messages, ratelimit};
use context::page_status::PageId;
use macros::handler;
use slint::SharedString;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tracing::Span;

pub struct ProcessSnapshotActor<P: UiProcessesPort, TWindow: Window> {
    pub snapshots: HashMap<&'static str, BridgeSnapshot>,
    pub contexts: HashMap<&'static str, Arc<StatefulContext>>,
    pub target: Addr<ProcessActor<P>, TWindow>,

    pub page_id: PageId,
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

impl<P: UiProcessesPort, TWindow: Window> ProcessSnapshotActor<P, TWindow> {
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
        Span::current().record("total_pids", total_count);

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

        ratelimit!(
            3600,
            info!(
                pids = total_count,
                cols = column_defs.len(),
                "Snapshot sent to UI"
            )
        );

        self.target.send(ProcessSnapshotReady {
            column_defs,
            processes: self.scratch_processes.clone(),
            total_count,
        });
    }
}

#[handler]
fn activate_page<P: UiProcessesPort, T: Window>(
    this: &mut ProcessSnapshotActor<P, T>,
    msg: PageActivated,
) {
    this.is_active = msg.page_id == this.page_id;
}

#[handler]
fn process_remote_scan<P: UiProcessesPort, T: Window>(
    this: &mut ProcessSnapshotActor<P, T>,
    msg: RemoteScanResult,
) {
    if !this.is_active {
        return;
    }

    let ctx = this.context_for(msg.schema_id);
    let result = WslScanResult {
        processes: msg.processes,
        machine: msg.machine,
        ctx,
    };
    let snapshot = build_snapshot(&result);
    this.snapshots.insert(msg.schema_id, snapshot);
    this.rebuild_and_send();
}

#[cfg(target_os = "windows")]
#[handler]
fn process_windows_report<P: UiProcessesPort, T: Window>(
    this: &mut ProcessSnapshotActor<P, T>,
    msg: WindowsReportMessage,
) {
    if !this.is_active {
        return;
    }

    let ctx = this.context_for("windows");
    let result = WindowsScanResult { report: msg.0, ctx };
    let snapshot = build_snapshot(&result);
    this.snapshots.insert("windows", snapshot);
    this.rebuild_and_send();
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
    ctx.tick();
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
            exe_path: proc.exe_path(ctx),
            fields: fields.clone(),
            #[cfg(windows)]
            package_name: proc.package_name(ctx),
        });
    });

    BridgeSnapshot {
        column_defs,
        processes,
    }
}
