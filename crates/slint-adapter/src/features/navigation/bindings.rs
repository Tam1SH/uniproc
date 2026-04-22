use crate::features::navigation::UiNavigationAdapter;
use app_contracts::features::navigation::UiNavigationBindings;
use macros::slint_bindings_adapter;
use slint::ComponentHandle;

#[slint_bindings_adapter(window = AppWindow)]
impl UiNavigationBindings for UiNavigationAdapter {
    fn on_request_route_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_request_route_switch(move |route_segment| handler(route_segment.to_string()));
    }

    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_request_tab_switch(move |context_key| handler(context_key.to_string()));
    }

    fn on_request_tab_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_request_tab_close(move |context_key| handler(context_key.to_string()));
    }

    fn on_request_tab_add<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_request_tab_add(move |context_key| handler(context_key.to_string()));
    }
}
