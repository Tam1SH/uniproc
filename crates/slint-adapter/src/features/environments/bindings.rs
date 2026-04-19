use crate::features::environments::UiEnvironmentsAdapter;
use app_contracts::features::environments::UiEnvironmentsBindings;
use macros::slint_bindings_adapter;

#[slint_bindings_adapter(window = AppWindow)]
impl UiEnvironmentsBindings for UiEnvironmentsAdapter {}
