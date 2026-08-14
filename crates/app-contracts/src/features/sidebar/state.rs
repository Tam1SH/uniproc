use guinea_macros::reducer;

use super::messages::{Sidebar, SidebarMsg};

#[derive(Clone, PartialEq, Debug)]
pub struct SidebarState {
    pub open: bool,
    pub width: u64,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            open: true,
            width: 260,
        }
    }
}

#[reducer]
#[dispatch(Sidebar)]
pub fn sidebar_reducer(state: &mut SidebarState, msg: SidebarMsg) {
    match msg {
        SidebarMsg::Set { open, width } => {
            state.open = open;
            state.width = width;
        }
    }
}
