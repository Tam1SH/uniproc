use crate::AppWindow;

mod port;

#[derive(Clone)]
pub struct UiCosmeticsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl UiCosmeticsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
