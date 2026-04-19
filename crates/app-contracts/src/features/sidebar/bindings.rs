use macros::slint_bindings;

#[slint_bindings(global = "Sidebar")]
pub trait UiSidebarBindings: 'static {
    fn on_side_bar_width_changed<F>(&self, handler: F)
    where
        F: Fn(u64) + 'static;
}
