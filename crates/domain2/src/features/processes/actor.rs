use std::cmp::Ordering;

use app_contracts2::features::agents::{WindowsActionRequest, WindowsReportMessage};
use app_contracts2::features::processes::{ProcessRow, ProcessesMsg, ProcessesPort};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};
use uniproc_protocol::{ProcessCommand, WindowsRequest};
use uuid::Uuid;

pub struct ProcessesActor<P: ProcessesPort> {
    ui_port: P,
    rows: Vec<ProcessRow>,
    sort_column: String,
    descending: bool,
    selected: Option<u32>,
}

impl<P: ProcessesPort> std::fmt::Debug for ProcessesActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessesActor")
            .field("rows", &self.rows.len())
            .field("sort_column", &self.sort_column)
            .field("descending", &self.descending)
            .field("selected", &self.selected)
            .finish()
    }
}

impl<P: ProcessesPort> ProcessesActor<P> {
    pub fn new(ui_port: P) -> Self {
        Self {
            ui_port,
            rows: Vec::new(),
            sort_column: "cpu".to_string(),
            descending: true,
            selected: None,
        }
    }

    fn publish_rows(&self) {
        self.ui_port.send(ProcessesMsg::SetRows(self.rows.clone()));
    }

    fn resort(&mut self) {
        sort_rows(&mut self.rows, &self.sort_column, self.descending);
    }
}

fn sort_rows(rows: &mut [ProcessRow], column: &str, descending: bool) {
    rows.sort_by(|a, b| {
        let ord = match column {
            "name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            "cpu" => a.cpu_percent.partial_cmp(&b.cpu_percent).unwrap_or(Ordering::Equal),
            "memory" => a.memory_bytes.cmp(&b.memory_bytes),
            "disk" => a.disk_bytes.cmp(&b.disk_bytes),
            "net" => a.net_bytes.cmp(&b.net_bytes),
            _ => Ordering::Equal,
        };
        let ord = ord.then_with(|| a.name.cmp(&b.name)).then_with(|| a.pid.cmp(&b.pid));
        if descending { ord.reverse() } else { ord }
    });
}

#[actor_manifest]
impl<P: ProcessesPort> ManagedActor for ProcessesActor<P> {
    type Handlers = handlers!(
        bind {
            Sort(String),
            Select(u32),
            Terminate
        },
        @WindowsReportMessage
    );
}

#[handler]
fn on_windows_report<P: ProcessesPort>(this: &mut ProcessesActor<P>, msg: WindowsReportMessage) {
    this.rows = msg
        .0
        .processes
        .iter()
        .map(|p| ProcessRow {
            pid: p.pid,
            name: p.name.clone(),
            cpu_percent: p.cpu_percent,
            memory_bytes: p.working_set_kb * 1024,
            disk_bytes: p.disk_read_bytes + p.disk_write_bytes,
            net_bytes: p.net_rx_bytes + p.net_tx_bytes,
        })
        .collect();
    this.resort();
    this.publish_rows();
}

#[handler]
fn sort<P: ProcessesPort>(this: &mut ProcessesActor<P>, msg: Sort) {
    if this.sort_column == msg.0 {
        this.descending = !this.descending;
    } else {
        this.sort_column = msg.0;
        this.descending = true;
    }
    this.resort();
    this.ui_port.send(ProcessesMsg::SetSort {
        column: this.sort_column.clone(),
        descending: this.descending,
    });
    this.publish_rows();
}

#[handler]
fn select<P: ProcessesPort>(this: &mut ProcessesActor<P>, msg: Select) {
    this.selected = Some(msg.0);
    this.ui_port.send(ProcessesMsg::SetSelected(this.selected));
}

#[handler]
fn terminate<P: ProcessesPort>(this: &mut ProcessesActor<P>, _: Terminate) {
    let Some(pid) = this.selected else {
        return;
    };
    GlobalEventBus::instance().publish(WindowsActionRequest::new(
        Uuid::new_v4(),
        WindowsRequest::ProcessCommand(ProcessCommand::Kill { pid }),
    ));
    this.selected = None;
    this.ui_port.send(ProcessesMsg::SetSelected(None));
}
