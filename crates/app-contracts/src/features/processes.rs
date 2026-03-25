use slint::Image;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldDefDto {
    pub id: String,
    pub label: String,
    pub stat_text: String,
    pub stat_detail: Option<String>,
    pub show_indicator: bool,
    pub stat_numeric: f32,
    pub threshold: f32,
    pub width_px: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessFieldDto {
    pub id: String,
    pub text: String,
    pub width_px: i32,
    pub numeric: f32,
    pub threshold: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessNodeDto {
    pub pid: u32,
    pub name: String,
    pub parent_pid: u32,
    pub exe_path: Option<String>,
    pub fields: Vec<ProcessFieldDto>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProcessEntryVm {
    pub pid: i32,
    pub name: String,
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

pub trait ProcessesUiPort: 'static {
    fn set_process_rows_window(&self, total_rows: usize, start: usize, rows: Vec<ProcessEntryVm>);
    fn set_column_defs(&self, defs: Vec<FieldDefDto>);
    fn set_loading(&self, loading: bool);
    fn get_selected_pid(&self) -> i32;
    fn set_selected_pid(&self, pid: i32);
    fn set_selected_name(&self, name: String);
    fn set_sort_state(&self, field: String, descending: bool);
    fn set_total_processes_count(&self, count: usize);
}

pub trait ProcessesUiBindings: 'static {
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(String) + 'static;

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static;

    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;

    fn on_rows_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;
}
