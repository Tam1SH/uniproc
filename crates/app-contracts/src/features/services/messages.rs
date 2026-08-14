use guinea_core::messages;
use guinea_macros::port;
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

messages! {
    pub Services {
        Sort(String),
        Select(String),
        Deselect,
        Command(ServiceActionKind),
    }
}
