use crate::AppWindow;

mod port;

#[derive(Clone)]
pub struct CosmeticsAdapter {
    ui: slint::Weak<AppWindow>,
}

impl CosmeticsAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}
