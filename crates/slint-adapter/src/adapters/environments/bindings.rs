use slint::ComponentHandle;
use crate::adapters::environments::EnvironmentsUiAdapter;
use app_contracts::features::environments::UiEnvironmentsBindings;
use macros::slint_bindings_adapter;

#[slint_bindings_adapter(window = AppWindow)]
impl UiEnvironmentsBindings for EnvironmentsUiAdapter {}
