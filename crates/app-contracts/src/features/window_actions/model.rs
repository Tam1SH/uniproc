use app_core::actor::traits::Message;
use macros::slint_dto;
use serde::{Deserialize, Serialize};

#[slint_dto]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WindowBreakpoint {
    Sm,
    Md,
    Lg,
}

#[derive(Debug, Clone)]
pub struct WindowConfigChanged {
    pub breakpoint: WindowBreakpoint,
    pub width: u64,
}

impl Message for WindowConfigChanged {}

#[slint_dto]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeEdge {
    North,
    South,
    West,
    East,
    NorthWest,
    NorthEast,
    SouthWest,
    SouthEast,
}
