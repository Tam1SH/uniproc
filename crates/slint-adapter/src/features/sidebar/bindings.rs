use crate::features::sidebar::UiSidebarAdapter;
use app_contracts::features::sidebar::UiSidebarBindings;
use macros::slint_bindings_adapter;
use slint::ComponentHandle;

#[slint_bindings_adapter(window = AppWindow)]
impl UiSidebarBindings for UiSidebarAdapter {
    fn on_side_bar_width_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(u64) + 'static,
    {
        ui.global::<crate::Sidebar>()
            .on_side_bar_width_changed(move |w| handler(w as u64));
    }
}
