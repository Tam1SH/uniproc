use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct NavigationUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl NavigationUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
