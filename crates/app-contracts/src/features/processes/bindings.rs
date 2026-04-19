use macros::slint_bindings;
use slint::SharedString;

#[slint_bindings(global = "ProcessesFeatureGlobal")]
pub trait UiProcessesBindings: 'static {
    #[tracing(target = "field")]
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;

    #[tracing(target = "group")]
    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    #[tracing(target = "pid,idx")]
    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;

    #[tracing(target = "start,count")]
    fn on_rows_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;

    #[tracing(target = "id,width")]
    fn on_column_resized<F>(&self, handler: F)
    where
        F: Fn(SharedString, f32) + 'static;

    fn on_group_clicked<F>(&self, handler: F)
    where
        F: Fn() + 'static;
}
