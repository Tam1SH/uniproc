use crate::{AppWindow, MainBodyState, ProcessEntry, ProcessField, ProcessesFeatureGlobal};
use app_contracts::features::processes::{
    FieldDefDto, ProcessEntryVm, ProcessFieldDto, ProcessesUiBindings, ProcessesUiPort,
};
use app_core::app::FromUiWeak;
use slint::{ComponentHandle, Model, ModelRc, VecModel};
use std::rc::Rc;

struct AdapterModels {
    rows: Rc<VecModel<ProcessEntry>>,
    columns: Rc<VecModel<crate::FieldDef>>,
}

#[derive(Clone)]
pub struct ProcessesUiAdapter {
    ui: slint::Weak<AppWindow>,
    models: Rc<AdapterModels>,
}

impl ProcessesUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        let models = Rc::new(AdapterModels {
            rows: Rc::new(VecModel::from(Vec::<ProcessEntry>::new())),
            columns: Rc::new(VecModel::from(Vec::<crate::FieldDef>::new())),
        });

        if let Some(window) = ui.upgrade() {
            let bridge = window.global::<ProcessesFeatureGlobal>();
            bridge.set_process_rows(models.rows.clone().into());
            bridge.set_column_defs(models.columns.clone().into());
        }

        Self { ui, models }
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

fn to_field_vm(field: ProcessFieldDto) -> ProcessField {
    ProcessField {
        id: field.id.into(),
        text: field.text.into(),
        width_px: field.width_px,
        numeric: field.numeric,
        threshold: field.threshold,
    }
}

fn to_entry_vm(entry: ProcessEntryVm) -> ProcessEntry {
    ProcessEntry {
        pid: entry.pid,
        name: entry.name.into(),
        icon: entry.icon,
        depth: entry.depth,
        has_children: entry.has_children,
        is_expanded: entry.is_expanded,
        is_dead: entry.is_dead,
        fields: ModelRc::new(VecModel::from(
            entry
                .fields
                .into_iter()
                .map(to_field_vm)
                .collect::<Vec<_>>(),
        )),
    }
}

impl ProcessesUiPort for ProcessesUiAdapter {
    fn set_process_rows_window(&self, total_rows: usize, start: usize, rows: Vec<ProcessEntryVm>) {
        if self.models.rows.row_count() != total_rows {
            let mut placeholders = Vec::with_capacity(total_rows);
            for _ in 0..total_rows {
                placeholders.push(ProcessEntry::default());
            }
            self.models.rows.set_vec(placeholders);
        }

        for (offset, row) in rows.into_iter().enumerate() {
            let idx = start + offset;
            if idx >= total_rows {
                break;
            }
            self.models.rows.set_row_data(idx, to_entry_vm(row));
        }
    }

    fn set_column_defs(&self, defs: Vec<FieldDefDto>) {
        let defs = defs
            .into_iter()
            .map(crate::FieldDef::from)
            .collect::<Vec<_>>();
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

    fn set_selected_name(&self, name: String) {
        self.with_ui(|ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .set_selected_name(name.into())
        });
    }

    fn set_sort_state(&self, field: String, descending: bool) {
        self.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_current_sort(field.into());
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
        F: Fn(String) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_sort_by(move |field| handler(field.to_string()));
        });
    }

    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(String) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_toggle_expand_group(move |group| handler(group.to_string()));
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
}
