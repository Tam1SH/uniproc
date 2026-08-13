use guinea_macros::{actions, port, reducer};

#[derive(Clone, Copy)]
pub enum SidebarMsg {
    Set { open: bool, width: u64 },
}

#[port]
pub trait SidebarPort: 'static {
    fn send(&self, msg: SidebarMsg);
}

#[actions]
pub trait SidebarActions {
    fn on_toggle<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_set_width<F>(&self, handler: F)
    where
        F: Fn(u64) + 'static;

    fn on_set_open<F>(&self, handler: F)
    where
        F: Fn(bool) + 'static;
}

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
#[dispatch(SidebarActions)]
pub fn sidebar_reducer(state: &mut SidebarState, msg: SidebarMsg) {
    match msg {
        SidebarMsg::Set { open, width } => {
            state.open = open;
            state.width = width;
        }
    }
}
