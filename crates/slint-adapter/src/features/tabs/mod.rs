use crate::AppWindow;

mod bindings;
mod port;

#[derive(Clone)]
pub struct UiTabsAdapter {
    pub(crate) ui: slint::Weak<AppWindow>,
}

impl UiTabsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
