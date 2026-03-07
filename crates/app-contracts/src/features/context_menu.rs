use crate::features::cosmetics::AccentColor;

pub trait ContextMenuUiPort: 'static {
    fn set_menu_open(&self, is_open: bool);
    fn invoke_terminate_selected(&self);
    fn on_action<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;
    fn show_menu(&self, x: f32, y: f32, reveal_delay_ms: u64);
    fn hide_menu(&self);
    fn set_menu_accent(&self, accent: AccentColor);
}

pub trait ContextMenuUiBindings: 'static {
    fn on_show_context_menu<F>(&self, handler: F)
    where
        F: Fn(f32, f32) + 'static;

    fn on_close_menu<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}
