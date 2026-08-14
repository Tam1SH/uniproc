use guinea_core::messages;
use guinea_macros::port;

#[derive(Clone, Copy)]
pub enum SidebarMsg {
    Set { open: bool, width: u64 },
}

#[port]
pub trait SidebarPort: 'static {
    fn send(&self, msg: SidebarMsg);
}

messages! {
    pub Sidebar {
        Toggle,
        SetOpen(bool),
        SetWidth(u64),
    }
}
