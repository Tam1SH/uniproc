use std::cmp::Ordering;
use std::rc::Rc;

use app_contracts2::features::agents::{WindowsAction, WindowsActionRequest, WindowsReportMessage};
use app_contracts2::features::processes::{
    MachineSummary, ProcessRow, ProcessesMsg, ProcessesPort,
};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};
use uuid::Uuid;

pub struct ProcessesActor<P: ProcessesPort> {
    ui_port: P,
    rows: Rc<[ProcessRow]>,
    machine_summary: MachineSummary,
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
            rows: Rc::from(Vec::new()),
            machine_summary: MachineSummary::default(),
            sort_column: "cpu".to_string(),
            descending: true,
            selected: None,
        }
    }

    fn publish_rows(&self) {
        self.ui_port.send(ProcessesMsg::SetRows {
            rows: self.rows.clone(),
            machine: self.machine_summary.clone(),
        });
    }

    fn resort(&mut self) {
        let pinned_pos = self
            .selected
            .and_then(|pid| self.rows.iter().position(|r| r.pid == pid));
        // `self.rows` is shared with whatever `SetRows` message the UI last
        // received (that's the whole point of the `Rc` - the common case,
        // publishing, is a refcount bump instead of a deep clone), so it
        // can't be sorted in place here; `to_vec()` is the one clone this
        // path pays, and only on a user-triggered sort click, not per tick.
        let mut rows = self.rows.to_vec();
        sort_rows_pinned(
            &mut rows,
            &self.sort_column,
            self.descending,
            pinned_pos,
            self.selected,
        );
        self.rows = Rc::from(rows);
    }
}

fn sort_rows_pinned(
    rows: &mut Vec<ProcessRow>,
    column: &str,
    descending: bool,
    pinned_pos: Option<usize>,
    pinned_pid: Option<u32>,
) {
    rows.sort_by(|a, b| {
        let ord = match column {
            "name" => a
                .name
                .chars()
                .flat_map(char::to_lowercase)
                .cmp(b.name.chars().flat_map(char::to_lowercase)),
            "cpu" => a
                .cpu_percent
                .partial_cmp(&b.cpu_percent)
                .unwrap_or(Ordering::Equal),
            "memory" => a.memory_bytes.cmp(&b.memory_bytes),
            "disk" => a.disk_bytes.cmp(&b.disk_bytes),
            "net" => a.net_bytes.cmp(&b.net_bytes),
            _ => Ordering::Equal,
        };
        let ord = ord
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.pid.cmp(&b.pid));
        if descending { ord.reverse() } else { ord }
    });

    if let (Some(pos), Some(pid)) = (pinned_pos, pinned_pid)
        && let Some(current) = rows.iter().position(|r| r.pid == pid)
    {
        let row = rows.remove(current);
        let insert_at = pos.min(rows.len());
        rows.insert(insert_at, row);
    }
}

#[actor_manifest]
impl<P: ProcessesPort> ManagedActor for ProcessesActor<P> {
    type Handlers = handlers!(
        bind {
            Sort(String),
            Select(u32),
            Deselect,
            Terminate
        },
        @WindowsReportMessage
    );
}

#[handler]
fn on_windows_report<P: ProcessesPort>(this: &mut ProcessesActor<P>, msg: WindowsReportMessage) {
    let machine = &msg.0.machine;
    this.machine_summary = MachineSummary {
        cpu_percent: machine.cpu_percent,
        cpu_current_mhz: machine.cpu_current_mhz,
        cpu_max_mhz: machine.cpu_max_mhz,
        memory_used_bytes: machine.used_physical_kb * 1024,
        memory_total_bytes: machine.total_physical_kb * 1024,
    };

    let pinned_pos = this
        .selected
        .and_then(|pid| this.rows.iter().position(|r| r.pid == pid));
    let pinned_pid = this.selected;

    let mut rows: Vec<ProcessRow> = msg
        .0
        .processes
        .into_iter()
        .map(|mut p| ProcessRow {
            pid: p.pid,
            cpu_percent: p.cpu_percent,
            memory_bytes: p.working_set_kb * 1024,
            disk_bytes: p.disk_read_bytes + p.disk_write_bytes,
            net_bytes: p.net_rx_bytes + p.net_tx_bytes,
            exe_path: if p.cmdline.is_empty() {
                String::new()
            } else {
                p.cmdline.swap_remove(0)
            },
            name: p.name,
            package_full_name: p.package_full_name,
        })
        .collect();
    sort_rows_pinned(
        &mut rows,
        &this.sort_column,
        this.descending,
        pinned_pos,
        pinned_pid,
    );
    this.rows = Rc::from(rows);
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
fn deselect<P: ProcessesPort>(this: &mut ProcessesActor<P>, _: Deselect) {
    this.selected = None;
    this.ui_port.send(ProcessesMsg::SetSelected(None));
}

#[handler]
fn terminate<P: ProcessesPort>(this: &mut ProcessesActor<P>, _: Terminate) {
    let Some(pid) = this.selected else {
        return;
    };
    GlobalEventBus::publish(WindowsActionRequest::new(
        Uuid::new_v4(),
        WindowsAction::Kill { pid },
    ));
    this.selected = None;
    this.ui_port.send(ProcessesMsg::SetSelected(None));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str, cpu: f32) -> ProcessRow {
        ProcessRow {
            pid,
            name: name.to_string(),
            cpu_percent: cpu,
            memory_bytes: 0,
            disk_bytes: 0,
            net_bytes: 0,
            exe_path: String::new(),
            package_full_name: String::new(),
        }
    }

    #[test]
    fn pinned_row_keeps_position_on_sort() {
        let mut rows = vec![
            row(1, "alpha", 5.0),
            row(2, "beta", 3.0),
            row(3, "gamma", 1.0),
        ];

        // Sort by cpu descending with pid 2 pinned at index 1.
        sort_rows_pinned(&mut rows, "cpu", true, Some(1), Some(2));

        assert_eq!(rows[1].pid, 2, "pinned row must stay at index 1");
        assert_eq!(rows[0].pid, 1, "highest cpu must be first");
        assert_eq!(rows[2].pid, 3, "lowest cpu must be last");
    }

    #[test]
    fn pinned_row_keeps_position_when_report_reorders() {
        // New report arrives in a different raw order: pinned pid 2 is at index 2,
        // but it was pinned at index 1 in the previous snapshot.
        let mut rows = vec![
            row(3, "gamma", 1.0),
            row(1, "alpha", 5.0),
            row(2, "beta", 3.0),
        ];

        sort_rows_pinned(&mut rows, "cpu", true, Some(1), Some(2));

        assert_eq!(rows[1].pid, 2, "pinned row must return to its pinned index");
        assert_eq!(rows[0].pid, 1);
        assert_eq!(rows[2].pid, 3);
    }

    #[test]
    fn pinned_row_keeps_position_on_repeated_sorts() {
        let mut rows = vec![
            row(1, "alpha", 5.0),
            row(2, "beta", 3.0),
            row(3, "gamma", 1.0),
        ];

        sort_rows_pinned(&mut rows, "cpu", true, Some(1), Some(2));
        assert_eq!(rows[1].pid, 2);

        // Simulate metric drift: beta now has the lowest cpu.
        for r in &mut rows {
            if r.pid == 2 {
                r.cpu_percent = 0.5;
            }
            if r.pid == 3 {
                r.cpu_percent = 4.0;
            }
        }

        sort_rows_pinned(&mut rows, "cpu", true, Some(1), Some(2));
        assert_eq!(rows[1].pid, 2, "pinned row must not drift across re-sorts");
        assert_eq!(rows[0].pid, 1);
        assert_eq!(rows[2].pid, 3);
    }

    #[test]
    fn missing_pinned_pos_sorts_normally() {
        let mut rows = vec![
            row(3, "gamma", 1.0),
            row(1, "alpha", 5.0),
            row(2, "beta", 3.0),
        ];

        sort_rows_pinned(&mut rows, "cpu", true, None, None);

        assert_eq!(rows[0].pid, 1);
        assert_eq!(rows[1].pid, 2);
        assert_eq!(rows[2].pid, 3);
    }
}
