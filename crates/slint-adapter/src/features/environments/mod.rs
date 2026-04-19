use crate::AppWindow;

mod bindings;
mod port;
#[derive(Clone)]
pub struct UiEnvironmentsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl UiEnvironmentsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
