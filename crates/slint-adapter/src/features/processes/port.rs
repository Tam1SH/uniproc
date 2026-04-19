use crate::AppWindow;
use app_contracts::features::processes::{
    FieldDefDto, FieldMetadata, ProcessEntryVm, UiProcessesPort,
};
use macros::slint_port_adapter;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use widgets::table::ui_cache::{SlintTableRowAdapter, UiTableCache};

struct AdapterModels {
    rows: Rc<VecModel<crate::ProcessEntry>>,
    columns: Rc<VecModel<crate::TableColDef>>,
    widths_model: Rc<VecModel<crate::TableColWidth>>,
    metadata_model: Rc<VecModel<crate::TableColMetadata>>,
    last_widths: RefCell<Vec<crate::TableColWidth>>,
    last_metadata: RefCell<Vec<crate::TableColMetadata>>,
}

#[derive(Clone)]
pub struct UiProcessesAdapter {
    pub ui: slint::Weak<AppWindow>,
    models: Rc<AdapterModels>,
    cache: Rc<RefCell<UiTableCache<crate::ProcessEntry, crate::TableCellData>>>,
}

impl UiProcessesAdapter {
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
            let bridge = window.global::<crate::ProcessesFeatureGlobal>();
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

#[slint_port_adapter(window = AppWindow)]
impl UiProcessesPort for UiProcessesAdapter {
    fn set_column_widths(&self, ui: &AppWindow, widths: Vec<(SharedString, u64)>) {
        let global = ui.global::<crate::ProcessesFeatureGlobal>();
        let defs = global.get_column_defs();
        let width_map: HashMap<SharedString, u64> = widths.into_iter().collect();

        let next_widths: Vec<crate::TableColWidth> = defs
            .iter()
            .map(|def| {
                let w = width_map.get(&def.id).cloned().unwrap_or(100);
                crate::TableColWidth {
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
        let global = ui.global::<crate::ProcessesFeatureGlobal>();
        let defs = global.get_column_defs();
        let data_map: HashMap<SharedString, FieldMetadata> =
            data.into_iter().map(|m| (m.id.clone(), m)).collect();

        let next_metadata: Vec<crate::TableColMetadata> = defs
            .iter()
            .map(|def| {
                if let Some(m) = data_map.get(&def.id) {
                    crate::TableColMetadata {
                        id: m.id.clone(),
                        is_text: m.is_text,
                        is_metric: m.is_metric,
                    }
                } else {
                    crate::TableColMetadata {
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
                .set_vec(vec![crate::ProcessEntry::default(); total_rows]);
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
        let defs = defs
            .into_iter()
            .map(crate::TableColDef::from)
            .collect::<Vec<_>>();
        self.models.columns.set_vec(defs);
    }

    fn get_selected_pid(&self, ui: &AppWindow) -> i32 {
        ui.global::<crate::ProcessesFeatureGlobal>()
            .get_selected_pid()
    }

    fn set_sort_state(&self, ui: &AppWindow, field: SharedString, descending: bool) {
        let bridge = ui.global::<crate::ProcessesFeatureGlobal>();
        bridge.set_current_sort(field);
        bridge.set_current_sort_descending(descending);
    }

    fn set_total_processes_count(&self, ui: &AppWindow, count: usize) {
        ui.global::<crate::ProcessesFeatureGlobal>()
            .set_total_processes_count(count as i32);
    }
}

impl SlintTableRowAdapter<crate::ProcessEntry, crate::TableCellData> for ProcessEntryVm {
    fn unique_id(&self) -> String {
        format!("{}-{}", self.pid, self.name)
    }

    fn to_slint_row(&self, cells: slint::ModelRc<crate::TableCellData>) -> crate::ProcessEntry {
        crate::ProcessEntry {
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

    fn update_slint_fields(&self, model: &Rc<VecModel<crate::TableCellData>>) {
        let cells: Vec<crate::TableCellData> = self
            .fields
            .iter()
            .map(|f| crate::TableCellData {
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

impl std::fmt::Debug for UiProcessesAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ProcessesUiAdapter");

        if let Some(ui) = self.ui.upgrade() {
            let g = ui.global::<crate::ProcessesFeatureGlobal>();
            debug
                .field(
                    "column_defs",
                    &g.get_column_defs().iter().collect::<Vec<_>>(),
                )
                .field(
                    "column_widths",
                    &g.get_column_widths().iter().collect::<Vec<_>>(),
                )
                .field(
                    "column_metadata",
                    &g.get_column_metadatas().iter().collect::<Vec<_>>(),
                )
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

impl From<FieldDefDto> for crate::TableColDef {
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

fn patch_model<T: Clone + 'static>(model: &Rc<VecModel<T>>, next: Vec<T>) {
    if model.row_count() != next.len() {
        model.set_vec(next);
        return;
    }
    for (i, item) in next.into_iter().enumerate() {
        model.set_row_data(i, item);
    }
}
