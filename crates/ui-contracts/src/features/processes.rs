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
    pub exe_path: Option<SharedString>,
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

pub trait ProcessesUiPort: Debug + 'static {
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>);
    fn set_column_metadata(&self, data: Vec<FieldMetadata>);
    fn set_process_rows_window(&self, total_rows: usize, start: usize, rows: &[ProcessEntryVm]);
    fn set_column_defs(&self, defs: Vec<FieldDefDto>);
    fn set_loading(&self, loading: bool);
    fn get_selected_pid(&self) -> i32;
    fn set_selected_pid(&self, pid: i32);
    fn set_selected_name(&self, name: SharedString);
    fn set_sort_state(&self, field: SharedString, descending: bool);
    fn set_total_processes_count(&self, count: usize);
}

pub trait ProcessesUiBindings: 'static {
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;

    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static;

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;

    fn on_rows_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;

    fn on_column_resized<F>(&self, handler: F)
    where
        F: Fn(SharedString, f32) + 'static;
}
