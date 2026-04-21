use crate::ServicesFeatureGlobal;
use crate::features::services::UiServicesAdapter;
use app_contracts::features::services::{ServiceActionKind, ServiceEntryVm, UiServicesBindings};
use macros::slint_bindings_adapter;
use slint::{ComponentHandle, SharedString};

#[slint_bindings_adapter(window = AppWindow)]
impl UiServicesBindings for UiServicesAdapter {
    fn on_service_action<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, ServiceActionKind) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_service_action(move |name, action| {
                let kind = match action.as_str() {
                    "Start" => ServiceActionKind::Start,
                    "Stop" => ServiceActionKind::Stop,
                    "Restart" => ServiceActionKind::Restart,
                    "Pause" => ServiceActionKind::Pause,
                    "Resume" => ServiceActionKind::Resume,
                    _ => return,
                };
                handler(name, kind);
            });
    }
}
