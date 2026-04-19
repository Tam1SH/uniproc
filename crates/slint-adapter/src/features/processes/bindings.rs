use crate::features::processes::UiProcessesAdapter;
use app_contracts::features::processes::UiProcessesBindings;
use macros::slint_bindings_adapter;

#[slint_bindings_adapter(window = AppWindow)]
impl UiProcessesBindings for UiProcessesAdapter {}
