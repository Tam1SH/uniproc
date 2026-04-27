use crate::features::tabs::UiTabsAdapter;
use app_contracts::features::tabs::UiTabsBindings;
use macros::slint_bindings_adapter;
use slint::ComponentHandle;

#[slint_bindings_adapter(window = AppWindow)]
impl UiTabsBindings for UiTabsAdapter {
    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Tabs>()
            .on_request_tab_switch(move |context_key| handler(context_key.to_string()));
    }

    fn on_request_tab_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Tabs>()
            .on_request_tab_close(move |context_key| handler(context_key.to_string()));
    }

    fn on_request_tab_add<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(String) + 'static,
    {
        ui.global::<crate::Tabs>()
            .on_request_tab_add(move |context_key| handler(context_key.to_string()));
    }
}
