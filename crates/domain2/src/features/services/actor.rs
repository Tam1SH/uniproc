use std::rc::Rc;

use app_contracts2::features::agents::{ScanTick, WindowsAction, WindowsActionRequest};
use app_contracts2::features::services::{
    ServiceActionKind, ServiceRow, ServicesMsg, ServicesPort,
};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::actor::{AsyncContext, ManagedActor, Message};
use guinea_macros::{actor_manifest, handler};
use uuid::Uuid;

use super::scanner;

pub struct ServicesActor<P: ServicesPort> {
    ui_port: P,
    rows: Rc<[ServiceRow]>,
    sort_column: String,
    descending: bool,
    selected: Option<String>,
}

impl<P: ServicesPort> std::fmt::Debug for ServicesActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServicesActor")
            .field("rows", &self.rows.len())
            .field("sort_column", &self.sort_column)
            .field("descending", &self.descending)
            .field("selected", &self.selected)
            .finish()
    }
}

impl<P: ServicesPort> ServicesActor<P> {
    pub fn new(ui_port: P) -> Self {
        Self {
            ui_port,
            rows: Rc::from(Vec::new()),
            sort_column: "name".to_string(),
            descending: false,
            selected: None,
        }
    }

    fn publish_rows(&self) {
        self.ui_port.send(ServicesMsg::SetRows {
            rows: self.rows.clone(),
        });
    }

    fn resort(&mut self) {
        let mut rows = self.rows.to_vec();
        sort_rows(&mut rows, &self.sort_column, self.descending);
        self.rows = Rc::from(rows);
    }
}

fn sort_rows(rows: &mut [ServiceRow], column: &str, descending: bool) {
    rows.sort_by(|a, b| {
        let ord = match column {
            "status" => a.status.cmp(&b.status),
            "group" => a.group.cmp(&b.group),
            "pid" => a.pid.cmp(&b.pid),
            _ => a
                .name
                .chars()
                .flat_map(char::to_lowercase)
                .cmp(b.name.chars().flat_map(char::to_lowercase)),
        };
        let ord = ord.then_with(|| a.name.cmp(&b.name));
        if descending { ord.reverse() } else { ord }
    });
}

enum ScanResult {
    Rows(Vec<ServiceRow>),
    Failed,
}
impl Message for ScanResult {}

#[actor_manifest]
impl<P: ServicesPort> ManagedActor for ServicesActor<P> {
    type Handlers = handlers!(
        bind {
            Sort(String),
            Select(String),
            Deselect,
            Command(ServiceActionKind)
        },
        @ScanTick,
        @ScanResult
    );
}

#[handler]
async fn handle_scan_tick<P: ServicesPort>(ctx: AsyncContext<ServicesActor<P>>, _: ScanTick) {
    match scanner::scan_services() {
        Ok(rows) => ctx.send(ScanResult::Rows(rows)),
        Err(err) => {
            tracing::warn!(%err, "services scan failed");
            ctx.send(ScanResult::Failed);
        }
    }
}

#[handler]
fn on_scan_result<P: ServicesPort>(this: &mut ServicesActor<P>, msg: ScanResult) {
    let ScanResult::Rows(mut rows) = msg else {
        return;
    };
    sort_rows(&mut rows, &this.sort_column, this.descending);
    this.rows = Rc::from(rows);
    this.publish_rows();
}

#[handler]
fn sort<P: ServicesPort>(this: &mut ServicesActor<P>, msg: Sort) {
    if this.sort_column == msg.0 {
        this.descending = !this.descending;
    } else {
        this.sort_column = msg.0;
        this.descending = false;
    }
    this.resort();
    this.ui_port.send(ServicesMsg::SetSort {
        column: this.sort_column.clone(),
        descending: this.descending,
    });
    this.publish_rows();
}

#[handler]
fn select<P: ServicesPort>(this: &mut ServicesActor<P>, msg: Select) {
    this.selected = Some(msg.0.clone());
    this.ui_port.send(ServicesMsg::SetSelected(Some(msg.0)));
}

#[handler]
fn deselect<P: ServicesPort>(this: &mut ServicesActor<P>, _: Deselect) {
    this.selected = None;
    this.ui_port.send(ServicesMsg::SetSelected(None));
}

#[handler]
fn command<P: ServicesPort>(this: &mut ServicesActor<P>, msg: Command) {
    let Some(name) = this.selected.clone() else {
        return;
    };
    let action = match msg.0 {
        ServiceActionKind::Start => WindowsAction::ServiceStart { name },
        ServiceActionKind::Stop => WindowsAction::ServiceStop { name },
        ServiceActionKind::Pause => WindowsAction::ServicePause { name },
        ServiceActionKind::Resume => WindowsAction::ServiceResume { name },
        ServiceActionKind::Restart => WindowsAction::ServiceRestart { name },
    };
    GlobalEventBus::publish(WindowsActionRequest::new(Uuid::new_v4(), action));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, status: &str) -> ServiceRow {
        ServiceRow {
            name: name.to_string(),
            display_name: name.to_string(),
            pid: 0,
            status: status.to_string(),
            group: String::new(),
            description: String::new(),
        }
    }

    #[test]
    fn sorts_by_name_case_insensitively_by_default() {
        let mut rows = vec![row("beta", "Running"), row("Alpha", "Stopped")];
        sort_rows(&mut rows, "name", false);
        assert_eq!(rows[0].name, "Alpha");
        assert_eq!(rows[1].name, "beta");
    }

    #[test]
    fn sorts_by_status_when_requested() {
        let mut rows = vec![row("a", "Running"), row("b", "Paused")];
        sort_rows(&mut rows, "status", false);
        assert_eq!(rows[0].status, "Paused");
    }
}
