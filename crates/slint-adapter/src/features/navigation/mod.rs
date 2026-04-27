use crate::AppWindow;

mod bindings;

#[derive(Clone)]
pub struct UiNavigationAdapter {
    pub(crate) ui: slint::Weak<AppWindow>,
}

impl UiNavigationAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
