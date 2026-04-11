use crate::adapters::processes::ProcessesUiAdapter;
use crate::AppWindow;
use app_contracts::features::processes::ProcessesUiBindings;
use macros::slint_bindings_adapter;

#[slint_bindings_adapter(window = AppWindow)]
impl ProcessesUiBindings for ProcessesUiAdapter {}
