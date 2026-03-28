use crate::{
    AppWindow, MainBodyState, ProcessEntry, ProcessField, ProcessesFeatureGlobal, TableColDef,
    TableColMetadata, TableColWidth,
};
use app_contracts::features::processes::{
    FieldDefDto, FieldMetadata, ProcessEntryVm, ProcessesUiBindings, ProcessesUiPort,
};
use app_core::app::FromUiWeak;
use app_table::ui_cache::{SlintTableRowAdapter, UiTableCache};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::info;

struct AdapterModels {
    rows: Rc<VecModel<ProcessEntry>>,
    columns: Rc<VecModel<TableColDef>>,
}

#[derive(Clone)]
pub struct ProcessesUiAdapter {
    ui: slint::Weak<AppWindow>,
    models: Rc<AdapterModels>,
    cache: Rc<RefCell<UiTableCache<ProcessEntry, ProcessField>>>,
}

impl ProcessesUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        let models = Rc::new(AdapterModels {
            rows: Rc::new(VecModel::from(Vec::<ProcessEntry>::new())),
            columns: Rc::new(VecModel::from(Vec::<TableColDef>::new())),
        });

        if let Some(window) = ui.upgrade() {
            let bridge = window.global::<ProcessesFeatureGlobal>();
            bridge.set_process_rows(models.rows.clone().into());
            bridge.set_column_defs(models.columns.clone().into());
        }

        Self {
            ui,
            models,
            cache: Default::default(),
        }
    }

    fn with_ui<F>(&self, f: F)
    where
        F: FnOnce(&AppWindow),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui);
        }
    }
}

impl FromUiWeak<AppWindow> for ProcessesUiAdapter {
    fn from_ui_weak(ui: slint::Weak<AppWindow>) -> Self {
        Self::new(ui)
    }
}

impl ProcessesUiPort for ProcessesUiAdapter {
    fn set_column_widths(&self, widths: Vec<(SharedString, u64)>) {
        self.with_ui(|ui| {
            let defs = ui.global::<ProcessesFeatureGlobal>().get_column_defs();
            let id_order: Vec<SharedString> = defs.iter().map(|d| d.id.clone()).collect();

            let width_map: HashMap<SharedString, u64> = widths.into_iter().collect();

            let sorted_widths_vec: Vec<TableColWidth> = id_order
                .into_iter()
                .map(|id| {
                    let w = width_map.get(&id).cloned().unwrap_or(100);

                    TableColWidth {
                        id: id.into(),
                        width_px: w as i32,
                    }
                })
                .collect();

            let model = ModelRc::new(VecModel::from(sorted_widths_vec));
            ui.global::<ProcessesFeatureGlobal>()
                .set_column_widths(model);
        });
    }

    fn set_column_metadata(&self, data: Vec<FieldMetadata>) {
        self.with_ui(|ui| {
            let global = ui.global::<ProcessesFeatureGlobal>();

            let defs = global.get_column_defs();

            let mut data_map: HashMap<SharedString, FieldMetadata> =
                data.into_iter().map(|m| (m.id.clone(), m)).collect();

            let sorted_metadata: Vec<TableColMetadata> = defs
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
            info!("{:?}", sorted_metadata);
            global.set_column_metadatas(ModelRc::new(VecModel::from(sorted_metadata)));
        });
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

    fn set_loading(&self, loading: bool) {
        self.with_ui(|ui| ui.global::<MainBodyState>().set_is_loading(loading));
    }

    fn get_selected_pid(&self) -> i32 {
        if let Some(ui) = self.ui.upgrade() {
            return ui.global::<ProcessesFeatureGlobal>().get_selected_pid();
        }
        -1
    }

    fn set_selected_pid(&self, pid: i32) {
        self.with_ui(|ui| ui.global::<ProcessesFeatureGlobal>().set_selected_pid(pid));
    }

    fn set_selected_name(&self, name: SharedString) {
        self.with_ui(|ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .set_selected_name(name)
        });
    }

    fn set_sort_state(&self, field: SharedString, descending: bool) {
        self.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_current_sort(field);
            bridge.set_current_sort_descending(descending);
        });
    }

    fn set_total_processes_count(&self, count: usize) {
        self.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_total_processes_count(count as i32);
        })
    }
}

impl ProcessesUiBindings for ProcessesUiAdapter {
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_sort_by(move |field| handler(field));
        });
    }

    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_toggle_expand_group(move |group| handler(group));
        });
    }

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(move |ui| ui.global::<ProcessesFeatureGlobal>().on_terminate(handler));
    }

    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_select_process(move |pid, idx| handler(pid, idx));
        });
    }

    fn on_rows_viewport_changed<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_rows_viewport_changed(move |start, count| handler(start, count));
        });
    }

    fn on_column_resized<F>(&self, handler: F)
    where
        F: Fn(SharedString, f32) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_column_resized(move |start, count| handler(start, count));
        });
    }
}

impl SlintTableRowAdapter<ProcessEntry, ProcessField> for ProcessEntryVm {
    fn unique_id(&self) -> String {
        format!("{}-{}", self.pid, self.name)
    }

    fn to_slint_row(&self, fields: slint::ModelRc<ProcessField>) -> ProcessEntry {
        ProcessEntry {
            pid: self.pid,
            name: self.name.clone(),
            icon: self.icon.clone(),
            depth: self.depth,
            has_children: self.has_children,
            is_expanded: self.is_expanded,
            is_dead: self.is_dead,
            fields,
        }
    }

    fn update_slint_fields(&self, model: &Rc<VecModel<ProcessField>>) {
        if model.row_count() != self.fields.len() {
            let empty_fields = vec![ProcessField::default(); self.fields.len()];
            model.set_vec(empty_fields);
        }

        for (i, f_dto) in self.fields.iter().enumerate() {
            let new_field = ProcessField {
                id: f_dto.id.clone(),
                text: f_dto.text.clone(),
                numeric: f_dto.numeric,
                threshold: f_dto.threshold,
            };

            if model.row_data(i) != Some(new_field.clone()) {
                model.set_row_data(i, new_field);
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

impl From<FieldDefDto> for TableColDef {
    fn from(value: FieldDefDto) -> Self {
        Self {
            id: value.id.into(),
            label: value.label.into(),
            stat_text: value.stat_text.into(),
            stat_numeric: value.stat_numeric,
            threshold: value.threshold,
            stat_detail: value.stat_detail.map(Into::into).unwrap_or_default(),
            show_indicator: value.show_indicator,
        }
    }
}
