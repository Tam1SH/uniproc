use guinea_macros::{actions, port};
use std::rc::Rc;

use super::model::{ServiceActionKind, ServiceRow};

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
