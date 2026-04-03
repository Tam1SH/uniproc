use crate::{
    AppWindow, ProcessEntry, ProcessesFeatureGlobal, TableCellData, TableColDef, TableColMetadata,
    TableColWidth,
};
use app_contracts::features::processes::{
    FieldDefDto, FieldMetadata, ProcessEntryVm, ProcessesUiBindings, ProcessesUiPort,
};
use app_core::app::FromUiWeak;
use macros::ui_adapter;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use widgets::table::ui_cache::{SlintTableRowAdapter, UiTableCache};

struct AdapterModels {
    rows: Rc<VecModel<ProcessEntry>>,
    columns: Rc<VecModel<TableColDef>>,

    widths_model: Rc<VecModel<TableColWidth>>,
    metadata_model: Rc<VecModel<TableColMetadata>>,

    last_widths: RefCell<Vec<TableColWidth>>,
    last_metadata: RefCell<Vec<TableColMetadata>>,
}

#[derive(Clone)]
pub struct ProcessesUiAdapter {
    ui: slint::Weak<AppWindow>,
    models: Rc<AdapterModels>,
    cache: Rc<RefCell<UiTableCache<ProcessEntry, TableCellData>>>,
}

impl ProcessesUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        let models = Rc::new(AdapterModels {
            rows: Rc::new(VecModel::default()),
            columns: Rc::new(VecModel::default()),
            widths_model: Rc::new(VecModel::default()),
            metadata_model: Rc::new(VecModel::default()),
            last_widths: Default::default(),
            last_metadata: Default::default(),
        });

        if let Some(window) = ui.upgrade() {
            let bridge = window.global::<ProcessesFeatureGlobal>();
            bridge.set_process_rows(models.rows.clone().into());
            bridge.set_column_defs(models.columns.clone().into());
            bridge.set_column_widths(models.widths_model.clone().into());
            bridge.set_column_metadatas(models.metadata_model.clone().into());
        }

        Self {
            ui,
            models,
            cache: Default::default(),
        }
    }
}

impl FromUiWeak<AppWindow> for ProcessesUiAdapter {
    fn from_ui_weak(ui: slint::Weak<AppWindow>) -> Self {
        Self::new(ui)
    }
}

#[ui_adapter]
impl ProcessesUiPort for ProcessesUiAdapter {
    fn set_column_widths(&self, ui: &AppWindow, widths: Vec<(SharedString, u64)>) {
        let global = ui.global::<ProcessesFeatureGlobal>();
        let defs = global.get_column_defs();
        let width_map: HashMap<SharedString, u64> = widths.into_iter().collect();

        let next_widths: Vec<TableColWidth> = defs
            .iter()
            .map(|def| {
                let w = width_map.get(&def.id).cloned().unwrap_or(100);
                TableColWidth {
                    id: def.id.clone(),
                    width_px: w as i32,
                }
            })
            .collect();

        let mut last = self.models.last_widths.borrow_mut();
        if *last == next_widths {
            return;
        }

        *last = next_widths.clone();

        patch_model(&self.models.widths_model, next_widths);
    }

    fn set_column_metadata(&self, ui: &AppWindow, data: Vec<FieldMetadata>) {
        let global = ui.global::<ProcessesFeatureGlobal>();
        let defs = global.get_column_defs();
        let data_map: HashMap<SharedString, FieldMetadata> =
            data.into_iter().map(|m| (m.id.clone(), m)).collect();

        let next_metadata: Vec<TableColMetadata> = defs
            .iter()
            .map(|def| {
                if let Some(m) = data_map.get(&def.id) {
                    TableColMetadata {
                        id: m.id.clone(),
                        is_text: m.is_text,
                        is_metric: m.is_metric,
                    }
                } else {
                    TableColMetadata {
                        id: def.id.clone(),
                        is_text: false,
                        is_metric: false,
                    }
                }
            })
            .collect();

        let mut last = self.models.last_metadata.borrow_mut();
        if *last == next_metadata {
            return;
        }

        *last = next_metadata.clone();

        patch_model(&self.models.metadata_model, next_metadata);
    }

    fn set_process_rows_window(&self, total_rows: usize, start: usize, rows: &[ProcessEntryVm]) {
        let mut cache = self.cache.borrow_mut();

        if self.models.rows.row_count() != total_rows {
            self.models
                .rows
                .set_vec(vec![ProcessEntry::default(); total_rows]);
            cache.clear();
        }

        for (offset, row_dto) in rows.iter().enumerate() {
            let idx = start + offset;
            if idx < total_rows {
                let entry = cache.get_row(idx, row_dto);
                self.models.rows.set_row_data(idx, entry);
            }
        }
    }

    fn set_column_defs(&self, defs: Vec<FieldDefDto>) {
        let defs = defs.into_iter().map(TableColDef::from).collect::<Vec<_>>();
        self.models.columns.set_vec(defs);
    }

    fn set_is_grouped(&self, ui: &AppWindow, is_grouped: bool) {
        ui.global::<ProcessesFeatureGlobal>()
            .set_is_grouped(is_grouped);
    }

    #[default(-1)]
    fn get_selected_pid(&self, ui: &AppWindow) -> i32 {
        ui.global::<ProcessesFeatureGlobal>().get_selected_pid()
    }

    fn set_selected_pid(&self, ui: &AppWindow, pid: i32) {
        ui.global::<ProcessesFeatureGlobal>().set_selected_pid(pid);
    }

    fn set_selected_name(&self, ui: &AppWindow, name: SharedString) {
        ui.global::<ProcessesFeatureGlobal>()
            .set_selected_name(name);
    }

    fn set_sort_state(&self, ui: &AppWindow, field: SharedString, descending: bool) {
        let bridge = ui.global::<ProcessesFeatureGlobal>();
        bridge.set_current_sort(field);
        bridge.set_current_sort_descending(descending);
    }

    fn set_total_processes_count(&self, ui: &AppWindow, count: usize) {
        let bridge = ui.global::<ProcessesFeatureGlobal>();
        bridge.set_total_processes_count(count as i32);
    }
}

#[ui_adapter]
impl ProcessesUiBindings for ProcessesUiAdapter {
    #[ui_action(scope = "ui.processes.sort", target = "field")]
    fn on_sort_by<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>().on_sort_by(handler);
    }

    #[ui_action(scope = "ui.processes.toggle_group", target = "group")]
    fn on_toggle_expand_group<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>()
            .on_toggle_expand_group(handler);
    }

    #[ui_action(scope = "ui.processes.terminate")]
    fn on_terminate<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>().on_terminate(handler);
    }

    #[ui_action(scope = "ui.processes.select", target = "pid,idx")]
    fn on_select_process<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(i32, i32) + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>()
            .on_select_process(handler);
    }

    #[ui_action(scope = "ui.processes.viewport", target = "start,count")]
    fn on_rows_viewport_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(i32, i32) + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>()
            .on_rows_viewport_changed(handler);
    }

    #[ui_action(scope = "ui.processes.column_resized", target = "id,width")]
    fn on_column_resized<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, f32) + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>()
            .on_column_resized(handler);
    }

    #[ui_action(scope = "ui.processes.group_clicked")]
    fn on_group_clicked<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<ProcessesFeatureGlobal>()
            .on_group_clicked(handler);
    }
}

impl SlintTableRowAdapter<ProcessEntry, TableCellData> for ProcessEntryVm {
    fn unique_id(&self) -> String {
        format!("{}-{}", self.pid, self.name)
    }

    fn to_slint_row(&self, cells: slint::ModelRc<TableCellData>) -> ProcessEntry {
        ProcessEntry {
            pid: self.pid,
            name: self.name.clone(),
            icon: self.icon.clone(),
            depth: self.depth,
            has_children: self.has_children,
            is_expanded: self.is_expanded,
            is_dead: self.is_dead,
            cells,
        }
    }

    fn update_slint_fields(&self, model: &Rc<VecModel<TableCellData>>) {
        let cells: Vec<TableCellData> = self
            .fields
            .iter()
            .map(|f| TableCellData {
                text: f.text.clone(),
                value: f.numeric,
                threshold: f.threshold,
                has_metric: f.id == "memory" && self.depth == 0,
                dead: self.is_dead,
            })
            .collect();

        if model.row_count() != cells.len() {
            model.set_vec(cells);
            return;
        }
        for (i, cell) in cells.into_iter().enumerate() {
            if model.row_data(i) != Some(cell.clone()) {
                model.set_row_data(i, cell);
            }
        }
    }
}

impl std::fmt::Debug for ProcessesUiAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ProcessesUiAdapter");

        if let Some(ui) = self.ui.upgrade() {
            let g = ui.global::<ProcessesFeatureGlobal>();

            debug
                .field("column_defs", &self.fetch_column_defs(&g))
                .field("column_widths", &self.fetch_column_widths(&g))
                .field("column_metadata", &self.fetch_column_metadata(&g))
                .field("selected_pid", &g.get_selected_pid())
                .field("selected_name", &g.get_selected_name().as_str())
                .field(
                    "sort",
                    &format!(
                        "{} (desc: {})",
                        g.get_current_sort(),
                        g.get_current_sort_descending()
                    ),
                )
                .field("total_count", &g.get_total_processes_count())
                .field("rows_in_model", &self.models.rows.row_count());
        }

        debug.finish()
    }
}

impl ProcessesUiAdapter {
    fn fetch_column_defs(&self, g: &ProcessesFeatureGlobal) -> Vec<TableColDef> {
        g.get_column_defs().iter().collect()
    }

    fn fetch_column_widths(&self, g: &ProcessesFeatureGlobal) -> Vec<TableColWidth> {
        g.get_column_widths().iter().collect()
    }

    fn fetch_column_metadata(&self, g: &ProcessesFeatureGlobal) -> Vec<TableColMetadata> {
        g.get_column_metadatas().iter().collect()
    }
}

fn patch_model<T: Clone + 'static>(model: &Rc<VecModel<T>>, next: Vec<T>) {
    if model.row_count() != next.len() {
        model.set_vec(next);
        return;
    }
    for (i, item) in next.into_iter().enumerate() {
        model.set_row_data(i, item);
    }
}

impl From<FieldDefDto> for TableColDef {
    fn from(value: FieldDefDto) -> Self {
        Self {
            id: value.id,
            label: value.label,
            stat_text: value.stat_text,
            stat_numeric: value.stat_numeric,
            threshold: value.threshold,
            stat_detail: value.stat_detail.unwrap_or_default(),
            show_indicator: value.show_indicator,
        }
    }
}
