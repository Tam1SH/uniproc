use crate::features::cosmetics::AccentColor;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunTaskRequest {
    pub env_id: String,
    pub command: String,
}

pub trait RunTaskPort: Clone + 'static {
    fn on_open<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_drag<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_run_task<F>(&self, handler: F)
    where
        F: Fn(RunTaskRequest) + 'static;

    fn show_dialog(&self);
    fn hide_dialog(&self);
    fn drag_dialog_window(&self);
    fn apply_dialog_effects(&self);
    fn set_dialog_accent(&self, accent: AccentColor);
}
