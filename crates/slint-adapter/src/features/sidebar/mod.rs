use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct UiSidebarAdapter {
    ui: slint::Weak<AppWindow>,
}

impl UiSidebarAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
