use crate::{AppWindow, ServiceEntry, ServicesFeatureGlobal, TableCellData, TableColWidth};
use app_contracts::features::services::{
    ServiceActionKind, ServiceEntryVm, ServicesUiBindings, ServicesUiPort,
};
use app_core::app::FromUiWeak;
use macros::ui_adapter;
use slint::{ComponentHandle, Model, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use widgets::table::ui_cache::{SlintTableRowAdapter, UiTableCache};

struct AdapterModels {
    rows: Rc<VecModel<ServiceEntry>>,
    widths_model: Rc<VecModel<TableColWidth>>,
    last_widths: RefCell<Vec<TableColWidth>>,
}

#[derive(Clone)]
pub struct ServicesUiAdapter {
    ui: slint::Weak<AppWindow>,
    models: Rc<AdapterModels>,
    cache: Rc<RefCell<UiTableCache<ServiceEntry, TableCellData>>>,
}

impl ServicesUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        let models = Rc::new(AdapterModels {
            rows: Rc::new(VecModel::default()),
            widths_model: Rc::new(VecModel::default()),
            last_widths: Default::default(),
        });

        if let Some(window) = ui.upgrade() {
            let bridge = window.global::<ServicesFeatureGlobal>();
            bridge.set_service_rows(models.rows.clone().into());
            bridge.set_column_widths(models.widths_model.clone().into());
        }

        Self {
            ui,
            models,
            cache: Default::default(),
        }
    }
}

impl FromUiWeak<AppWindow> for ServicesUiAdapter {
    fn from_ui_weak(ui: slint::Weak<AppWindow>) -> Self {
        Self::new(ui)
    }
}

#[ui_adapter]
impl ServicesUiPort for ServicesUiAdapter {
    fn set_column_widths(&self, ui: &AppWindow, widths: Vec<(SharedString, u64)>) {
        let global = ui.global::<ServicesFeatureGlobal>();
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

        if self.models.widths_model.row_count() != next_widths.len() {
            self.models.widths_model.set_vec(next_widths);
        } else {
            for (i, item) in next_widths.into_iter().enumerate() {
                self.models.widths_model.set_row_data(i, item);
            }
        }
    }

    fn set_service_rows_window(&self, total_rows: usize, start: usize, rows: &[ServiceEntryVm]) {
        let mut cache = self.cache.borrow_mut();

        if self.models.rows.row_count() != total_rows {
            self.models
                .rows
                .set_vec(vec![ServiceEntry::default(); total_rows]);
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

    fn set_loading(&self, ui: &AppWindow, loading: bool) {
        ui.global::<ServicesFeatureGlobal>().set_is_loading(loading);
    }

    fn set_selected_name(&self, ui: &AppWindow, name: SharedString) {
        ui.global::<ServicesFeatureGlobal>().set_selected_name(name);
    }

    fn set_sort_state(&self, ui: &AppWindow, field: SharedString, descending: bool) {
        let bridge = ui.global::<ServicesFeatureGlobal>();
        bridge.set_current_sort(field);
        bridge.set_current_sort_descending(descending);
    }

    fn set_total_services_count(&self, ui: &AppWindow, count: usize) {
        ui.global::<ServicesFeatureGlobal>()
            .set_total_services_count(count as i32);
    }

    fn set_active_start_button(&self, ui: &AppWindow, flag: bool) {
        ui.global::<ServicesFeatureGlobal>()
            .set_start_button_active(flag);
    }

    fn set_active_stop_button(&self, ui: &AppWindow, flag: bool) {
        ui.global::<ServicesFeatureGlobal>()
            .set_stop_button_active(flag);
    }

    fn set_active_restart_button(&self, ui: &AppWindow, flag: bool) {
        ui.global::<ServicesFeatureGlobal>()
            .set_restart_button_active(flag);
    }
}

#[ui_adapter]
impl ServicesUiBindings for ServicesUiAdapter {
    fn on_sort_by<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>().on_sort_by(handler);
    }

    fn on_select_service<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, usize) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_select_service(move |name, idx| {
                handler(name, idx as usize);
            });
    }

    fn on_viewport_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(usize, usize) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_rows_viewport_changed(move |start, count| {
                handler(start.max(0) as usize, count.max(0) as usize);
            });
    }

    fn on_column_resized<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, f32) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_column_resized(handler);
    }

    fn on_service_action<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, ServiceActionKind) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_service_action(move |name, action| {
                let kind = match action.as_str() {
                    "Start" => ServiceActionKind::Start,
                    "Stop" => ServiceActionKind::Stop,
                    "Restart" => ServiceActionKind::Restart,
                    "Pause" => ServiceActionKind::Pause,
                    "Resume" => ServiceActionKind::Resume,
                    _ => return,
                };
                handler(name, kind);
            });
    }

    fn on_open_system_services<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_open_system_services(handler);
    }
}

impl SlintTableRowAdapter<ServiceEntry, TableCellData> for ServiceEntryVm {
    fn unique_id(&self) -> String {
        self.name.to_string()
    }

    fn to_slint_row(&self, cells: slint::ModelRc<TableCellData>) -> ServiceEntry {
        ServiceEntry {
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            pid: self.pid,
            status: self.status.clone(),
            group: self.group.clone(),
            description: self.description.clone(),
            cells,
        }
    }

    fn update_slint_fields(&self, model: &Rc<VecModel<TableCellData>>) {
        let cells = vec![
            TableCellData {
                text: if self.pid > 0 {
                    self.pid.to_string().into()
                } else {
                    "".into()
                },
                value: 0.0,
                threshold: 0.0,
                has_metric: false,
                dead: false,
            },
            TableCellData {
                text: self.status.clone(),
                value: 0.0,
                threshold: 0.0,
                has_metric: false,
                dead: false,
            },
            TableCellData {
                text: self.group.clone(),
                value: 0.0,
                threshold: 0.0,
                has_metric: false,
                dead: false,
            },
            TableCellData {
                text: self.description.clone(),
                value: 0.0,
                threshold: 0.0,
                has_metric: false,
                dead: false,
            },
        ];

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

impl std::fmt::Debug for ServicesUiAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServicesUiAdapter").finish()
    }
}
