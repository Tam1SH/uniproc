use guinea_core::Load;
use guinea_macros::{actions, port, reducer};
use std::rc::Rc;

#[derive(Clone, PartialEq, Debug)]
pub struct ServiceRow {
    pub name: String,
    pub display_name: String,
    pub pid: u32,
    pub status: String,
    pub group: String,
    pub description: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceActionKind {
    Start,
    Stop,
    Pause,
    Resume,
    Restart,
}

#[derive(Clone)]
pub enum ServicesMsg {
    SetRows { rows: Rc<[ServiceRow]> },
    SetSelected(Option<String>),
    SetSort { column: String, descending: bool },
}

#[port]
pub trait ServicesPort: 'static {
    fn send(&self, msg: ServicesMsg);
}

#[actions]
pub trait ServicesActions {
    fn on_sort<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_select<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_deselect<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_command<F>(&self, handler: F)
    where
        F: Fn(ServiceActionKind) + 'static;
}

#[derive(Clone, PartialEq, Debug)]
pub struct ServicesState {
    /// Agent reports as an async resource: `Load::Loading` until the first
    /// snapshot arrives. An empty vec inside `Ready` is a valid answer, not
    /// "still loading" - see `ProcessesState::rows` for the same shape.
    pub rows: Load<Rc<[ServiceRow]>>,
    pub selected: Option<String>,
    pub sort_column: String,
    pub descending: bool,
}

impl Default for ServicesState {
    fn default() -> Self {
        Self {
            rows: Load::Loading,
            selected: None,
            sort_column: "name".to_string(),
            descending: false,
        }
    }
}

impl ServicesState {
    pub fn rows(&self) -> &[ServiceRow] {
        self.rows.ready().map(|r| r.as_ref()).unwrap_or(&[])
    }

    pub fn total(&self) -> usize {
        self.rows().len()
    }
}

#[reducer]
#[dispatch(ServicesActions)]
pub fn services_reducer(state: &mut ServicesState, msg: ServicesMsg) {
    match msg {
        ServicesMsg::SetRows { rows } => {
            state.rows = Load::Ready(rows);
        }
        ServicesMsg::SetSelected(name) => {
            state.selected = name;
        }
        ServicesMsg::SetSort { column, descending } => {
            state.sort_column = column;
            state.descending = descending;
        }
    }
}
