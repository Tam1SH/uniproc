use crate::features::agents::WindowsServiceState;

#[derive(Clone, PartialEq, Debug)]
pub struct ServiceRow {
    pub name: String,
    pub display_name: String,
    pub pid: u32,
    pub state: WindowsServiceState,
    pub group: String,
    pub description: String,
    pub image_path: String,
}

impl ServiceRow {
    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ServiceActionKind {
    Start,
    Stop,
    Pause,
    Resume,
    Restart,
}
