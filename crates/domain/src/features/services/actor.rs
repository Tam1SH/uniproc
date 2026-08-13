use std::rc::Rc;

use app_contracts::features::agents::{
    WindowsAction, WindowsActionRequest, WindowsReportMessage, WindowsServiceStats,
};
use app_contracts::features::services::{
    ServiceActionKind, ServiceRow, ServicesMsg, ServicesPort,
};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};
use uuid::Uuid;

#[derive(derive_more::Debug)]
pub struct ServicesActor<P: ServicesPort> {
    #[debug(skip)]
    ui_port: P,
    #[debug("{}", rows.len())]
    rows: Rc<[ServiceRow]>,
    sort_column: String,
    descending: bool,
    selected: Option<String>,
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

fn to_row(svc: &WindowsServiceStats) -> ServiceRow {
    ServiceRow {
        name: svc.name.clone(),
        display_name: svc.display_name.clone(),
        pid: svc.pid,
        state: svc.state,
        group: svc.load_group.clone(),
        description: svc.description.clone(),
        image_path: svc.image_path.clone(),
    }
}

fn sort_rows(rows: &mut [ServiceRow], column: &str, descending: bool) {
    rows.sort_by(|a, b| {
        let ord = match column {
            "status" => a.state.id().cmp(b.state.id()),
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

#[actor_manifest]
impl<P: ServicesPort> ManagedActor for ServicesActor<P> {
    type Handlers = handlers!(
        bind {
            Sort(String),
            Select(String),
            Deselect,
            Command(ServiceActionKind)
        },
        @WindowsReportMessage
    );
}

#[handler]
fn on_windows_report<P: ServicesPort>(this: &mut ServicesActor<P>, msg: WindowsReportMessage) {
    let WindowsReportMessage::Report(report) = msg else {
        return;
    };
    let mut rows: Vec<ServiceRow> = report.services.iter().map(to_row).collect();
    sort_rows(&mut rows, &this.sort_column, this.descending);
    this.rows = Rc::from(rows);

    if let Some(selected) = &this.selected
        && !this.rows.iter().any(|r| &r.name == selected)
    {
        this.selected = None;
        this.ui_port.send(ServicesMsg::SetSelected(None));
    }

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

    use app_contracts::features::agents::WindowsServiceState;

    fn row(name: &str, state: WindowsServiceState) -> ServiceRow {
        ServiceRow {
            name: name.to_string(),
            display_name: name.to_string(),
            pid: 0,
            state,
            group: String::new(),
            description: String::new(),
            image_path: String::new(),
        }
    }

    #[test]
    fn sorts_by_name_case_insensitively_by_default() {
        let mut rows = vec![
            row("beta", WindowsServiceState::Running),
            row("Alpha", WindowsServiceState::Stopped),
        ];
        sort_rows(&mut rows, "name", false);
        assert_eq!(rows[0].name, "Alpha");
        assert_eq!(rows[1].name, "beta");
    }

    #[test]
    fn sorts_by_status_when_requested() {
        let mut rows = vec![
            row("a", WindowsServiceState::Running),
            row("b", WindowsServiceState::Paused),
        ];
        sort_rows(&mut rows, "status", false);
        assert_eq!(rows[0].state, WindowsServiceState::Paused);
    }
}
