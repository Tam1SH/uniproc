use crate::features::services::{ServicesPropertiesWindowUiAdapter, UiServicesAdapter};
use crate::{ServiceEntry, ServicesFeatureGlobal, TableCellData, TableColWidth};
use app_contracts::features::services::{ServiceEntryVm, UiServiceDetailsPort, UiServicesPort};
use macros::slint_port_adapter;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use widgets::table::ui_cache::SlintTableRowAdapter;

#[slint_port_adapter(window = ServicePropertiesDialogWindow)]
impl UiServiceDetailsPort for ServicesPropertiesWindowUiAdapter {
    fn set_selected_service_details(
        &self,
        ui: &ServicePropertiesDialogWindow,
        entry: ServiceEntryVm,
    ) {
        let g = ui.global::<ServicesFeatureGlobal>();
        g.set_selected_entry(entry.into());
    }
    fn set_active_buttons(
        &self,
        ui: &ServicePropertiesDialogWindow,
        start_button_active: bool,
        stop_button_active: bool,
        restart_button_active: bool,
    ) {
        let global = ui.global::<ServicesFeatureGlobal>();
        global.set_stop_button_active(stop_button_active);
        global.set_start_button_active(start_button_active);
        global.set_restart_button_active(restart_button_active);
    }
}

#[slint_port_adapter(window = AppWindow)]
impl UiServiceDetailsPort for UiServicesAdapter {
    fn set_selected_service_details(&self, ui: &AppWindow, entry: ServiceEntryVm) {
        let g = ui.global::<ServicesFeatureGlobal>();
        g.set_selected_entry(entry.into());
    }

    fn set_active_buttons(
        &self,
        ui: &AppWindow,
        start_button_active: bool,
        stop_button_active: bool,
        restart_button_active: bool,
    ) {
        let global = ui.global::<ServicesFeatureGlobal>();
        global.set_stop_button_active(stop_button_active);
        global.set_start_button_active(start_button_active);
        global.set_restart_button_active(restart_button_active);
    }
}

#[slint_port_adapter(window = AppWindow)]
impl UiServicesPort for UiServicesAdapter {
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
}

impl From<ServiceEntry> for ServiceEntryVm {
    fn from(entry: ServiceEntry) -> Self {
        Self {
            status: entry.status.clone(),
            name: entry.name.clone(),
            pid: entry.pid,
            description: entry.description.clone(),
            group: entry.group.clone(),
            display_name: entry.display_name.clone(),
        }
    }
}

impl From<ServiceEntryVm> for ServiceEntry {
    fn from(entry: ServiceEntryVm) -> Self {
        Self {
            status: entry.status.clone(),
            name: entry.name.clone(),
            pid: entry.pid,
            description: entry.description.clone(),
            group: entry.group.clone(),
            display_name: entry.display_name.clone(),
            cells: Default::default(),
        }
    }
}

impl SlintTableRowAdapter<ServiceEntry, TableCellData> for ServiceEntryVm {
    fn unique_id(&self) -> String {
        self.name.to_string()
    }

    fn to_slint_row(&self, cells: ModelRc<TableCellData>) -> ServiceEntry {
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
