use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct WindowActionsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl WindowActionsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
