use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct UiWindowActionsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl UiWindowActionsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
