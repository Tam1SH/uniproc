use slint::{Image, SharedString};

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
