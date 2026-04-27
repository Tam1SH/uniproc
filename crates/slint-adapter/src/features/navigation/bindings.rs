use crate::features::navigation::UiNavigationAdapter;
use app_contracts::features::navigation::UiNavigationBindings;
use macros::slint_bindings_adapter;
use slint::ComponentHandle;

#[slint_bindings_adapter(window = AppWindow)]
impl UiNavigationBindings for UiNavigationAdapter {
    fn on_push<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_push(move |path| handler(path.to_string()));
    }
}
