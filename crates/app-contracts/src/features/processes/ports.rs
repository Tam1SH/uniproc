use macros::slint_port;
use slint::SharedString;
use std::fmt::Debug;

use super::model::{FieldDefDto, FieldMetadata, ProcessEntryVm};

#[slint_port(global = "ProcessesFeatureGlobal")]
pub trait UiProcessesPort: Debug + 'static {
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
    fn set_empty_state_visible(&self, visible: bool);
    fn set_empty_state_title(&self, title: SharedString);
    fn set_empty_state_message(&self, message: SharedString);
    fn set_is_grouped(&self, is_grouped: bool);
    fn set_selected_pid(&self, pid: i32);
    fn set_selected_name(&self, name: SharedString);
}
