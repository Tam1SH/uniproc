use crate::AppWindow;

mod bindings;
mod port;
#[derive(Clone)]
pub struct EnvironmentsUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl EnvironmentsUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
