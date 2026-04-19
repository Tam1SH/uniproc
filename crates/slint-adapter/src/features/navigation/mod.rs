use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct UiNavigationAdapter {
    ui: slint::Weak<AppWindow>,
}

impl UiNavigationAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
