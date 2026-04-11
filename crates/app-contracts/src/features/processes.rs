use macros::{slint_bindings, slint_port};
use slint::{Image, SharedString};
use std::fmt::Debug;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldDefDto {
    pub id: SharedString,
    pub label: SharedString,
    pub stat_text: SharedString,
    pub stat_detail: Option<SharedString>,
    pub show_indicator: bool,
    pub stat_numeric: f32,
    pub threshold: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldMetadata {
    pub id: SharedString,
    pub is_text: bool,
    pub is_metric: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessFieldDto {
    pub id: SharedString,
    pub text: SharedString,
    pub numeric: f32,
    pub threshold: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessNodeDto {
    pub pid: u32,
    pub name: SharedString,
    pub parent_pid: u32,
    pub exe_path: SharedString,
    #[cfg(windows)]
    pub package_name: Option<SharedString>,
    pub fields: Vec<ProcessFieldDto>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessEntryVm {
    pub pid: i32,
    pub name: SharedString,
    pub icon: Image,
    pub depth: i32,
    pub has_children: bool,
    pub is_expanded: bool,
    pub is_dead: bool,
    pub fields: Vec<ProcessFieldDto>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessGroupVm {
    pub parent: ProcessEntryVm,
    pub children: Vec<ProcessEntryVm>,
}

#[slint_port(global = "ProcessesFeatureGlobal")]
pub trait ProcessesUiPort: Debug + 'static {
    #[manual]
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>);
    #[manual]
    fn set_column_metadata(&self, data: Vec<FieldMetadata>);
    #[manual]
    fn set_process_rows_window(&self, total_rows: usize, start: usize, rows: &[ProcessEntryVm]);
    #[manual]
    fn set_column_defs(&self, defs: Vec<FieldDefDto>);
    #[manual]
    fn get_selected_pid(&self) -> i32;
    #[manual]
    fn set_sort_state(&self, field: SharedString, descending: bool);
    #[manual]
    fn set_total_processes_count(&self, count: usize);
    fn set_is_grouped(&self, is_grouped: bool);
    fn set_selected_pid(&self, pid: i32);
    fn set_selected_name(&self, name: SharedString);
}

#[slint_bindings(global = "ProcessesFeatureGlobal")]
pub trait ProcessesUiBindings: 'static {
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
