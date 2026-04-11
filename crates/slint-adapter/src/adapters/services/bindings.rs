use crate::adapters::services::ServicesUiAdapter;
use crate::{AppWindow, ServicesFeatureGlobal};
use app_contracts::features::services::{UiServicesBindings, ServiceActionKind, ServiceEntryVm};
use macros::slint_bindings_adapter;
use slint::{SharedString, ComponentHandle};

#[slint_bindings_adapter(window = AppWindow)]
impl UiServicesBindings for ServicesUiAdapter {
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
