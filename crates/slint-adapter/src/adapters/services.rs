use crate::{
    AppWindow, ServiceEntry, ServicePropertiesDialogWindow, ServicesFeatureGlobal, TableCellData,
    TableColWidth, Theme,
};
use app_contracts::features::services::{
    ServiceActionKind, ServiceDetailsPort, ServiceEntryDto, ServiceEntryVm, ServicesUiBindings,
    ServicesUiPort, ServicesWindowRegister, PROPERTIES_DIALOG_KEY,
};
use app_core::actor::event_bus::EventBus;
use std::any::{Any, TypeId};

use context::native_windows::slint_factory::{OpenWindow, SlintWindowRegistry, WindowRegistry};
use context::native_windows::{NativeWindowConfig, NativeWindowManager, UiAdapter};
use i_slint_backend_winit::WinitWindowAccessor;
use macros::ui_adapter;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, LogicalSize, Model, SharedString, VecModel, WindowSize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::info;
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

impl ServicesWindowRegister for ServicesUiAdapter {
    fn register(&self, registry: &SlintWindowRegistry) {
        registry.register(PROPERTIES_DIALOG_KEY, || {
            let dialog = ServicePropertiesDialogWindow::new()
                .expect("service properties dialog window should initialize");

            dialog.on_drag_requested({
                let d = dialog.clone_strong();
                move || {
                    d.window().with_winit_window(|w| {
                        let _ = w.drag_window();
                    });
                }
            });

            dialog.on_close_requested({
                let d = dialog.clone_strong();
                move || {
                    d.window().dispatch_event(WindowEvent::CloseRequested);
                }
            });

            let theme = dialog.global::<Theme>();

            if let Ok(accent) = context::native_windows::platform::get_system_accent() {
                theme.set_accent(accent.into());
            }

            NativeWindowManager::with_config(
                dialog.clone_strong(),
                NativeWindowConfig::win11_dialog(),
            )
            .with_adapter(ServicesPropertiesWindowUiAdapter::new(dialog.as_weak()))
        });
    }
}

#[derive(Clone)]
pub struct ServicesPropertiesWindowUiAdapter {
    ui: slint::Weak<ServicePropertiesDialogWindow>,
}

impl UiAdapter for ServicesPropertiesWindowUiAdapter {
    fn query_port(&self, type_id: TypeId) -> Option<Box<dyn Any>> {
        if type_id == TypeId::of::<dyn ServiceDetailsPort>() {
            let port: Box<dyn ServiceDetailsPort> = Box::new(self.clone());
            return Some(Box::new(port) as Box<dyn Any>);
        }
        None
    }
    fn box_clone(&self) -> Box<dyn UiAdapter> {
        Box::new(self.clone())
    }
}

impl ServicesPropertiesWindowUiAdapter {
    pub fn new(ui: slint::Weak<ServicePropertiesDialogWindow>) -> Self {
        Self { ui }
    }
}

#[ui_adapter]
impl ServiceDetailsPort for ServicesPropertiesWindowUiAdapter {
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
        start_flag: bool,
        stop_flag: bool,
        restart_flag: bool,
    ) {
        let global = ui.global::<ServicesFeatureGlobal>();
        global.set_stop_button_active(stop_flag);
        global.set_start_button_active(start_flag);
        global.set_restart_button_active(restart_flag);
    }
}

#[ui_adapter]
impl ServiceDetailsPort for ServicesUiAdapter {
    fn set_selected_service_details(&self, ui: &AppWindow, entry: ServiceEntryVm) {
        let g = ui.global::<ServicesFeatureGlobal>();
        g.set_selected_entry(entry.into());
    }

    fn set_active_buttons(
        &self,
        ui: &AppWindow,
        start_flag: bool,
        stop_flag: bool,
        restart_flag: bool,
    ) {
        let global = ui.global::<ServicesFeatureGlobal>();
        global.set_stop_button_active(stop_flag);
        global.set_start_button_active(start_flag);
        global.set_restart_button_active(restart_flag);
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

    fn set_sort_state(&self, ui: &AppWindow, field: SharedString, descending: bool) {
        let bridge = ui.global::<ServicesFeatureGlobal>();
        bridge.set_current_sort(field);
        bridge.set_current_sort_descending(descending);
    }

    fn set_total_services_count(&self, ui: &AppWindow, count: usize) {
        ui.global::<ServicesFeatureGlobal>()
            .set_total_services_count(count as i32);
    }
}

#[ui_adapter]
impl ServicesUiBindings for ServicesUiAdapter {
    #[ui_action(scope = "ui.services.sort", target = "field")]
    fn on_sort_by<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>().on_sort_by(handler);
    }

    #[ui_action(scope = "ui.services.select", target = "name,idx")]
    fn on_select_service<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, usize) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_select_service(move |name, idx| {
                handler(name, idx as usize);
            });
    }

    #[ui_action(scope = "ui.services.viewport", target = "start,count")]
    fn on_viewport_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(usize, usize) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_rows_viewport_changed(move |start, count| {
                handler(start.max(0) as usize, count.max(0) as usize);
            });
    }

    #[ui_action(scope = "ui.services.column_resized", target = "id,width")]
    fn on_column_resized<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(SharedString, f32) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_column_resized(handler);
    }

    #[ui_action(scope = "ui.services.action", target = "name,kind")]
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

    #[ui_action(scope = "ui.services.open_system")]
    fn on_open_system_services<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_open_system_services(handler);
    }

    #[ui_action(scope = "ui.services.open_properties_window", target = "service-name")]
    fn on_open_properties_window<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(ServiceEntryVm) + 'static,
    {
        ui.global::<ServicesFeatureGlobal>()
            .on_open_properties_window(move |slint_entry| handler(slint_entry.into()));
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
