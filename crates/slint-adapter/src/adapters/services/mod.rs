use crate::{
    AppWindow, ServiceEntry, ServicePropertiesDialogWindow, ServicesFeatureGlobal, TableCellData,
    TableColWidth, Theme,
};
use app_contracts::features::services::{
    ServicesWindowRegister, UiServiceDetailsPort, PROPERTIES_DIALOG_KEY,
};
use context::native_windows::slint_factory::{SlintWindowRegistry, WindowRegistry};
use context::native_windows::{NativeWindowConfig, NativeWindowManager, UiAdapter};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::platform::WindowEvent;
use slint::{ComponentHandle, Model, VecModel};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;
use widgets::table::ui_cache::UiTableCache;

mod bindings;
mod port;

#[derive(Clone)]
pub struct ServicesUiAdapter {
    pub(crate) ui: slint::Weak<AppWindow>,
    pub(crate) models: Rc<AdapterModels>,
    pub(crate) cache: Rc<RefCell<UiTableCache<ServiceEntry, TableCellData>>>,
}

pub(crate) struct AdapterModels {
    pub(crate) rows: Rc<VecModel<ServiceEntry>>,
    pub(crate) widths_model: Rc<VecModel<TableColWidth>>,
    pub(crate) last_widths: RefCell<Vec<TableColWidth>>,
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

            if let Ok(accent_palette) = context::native_windows::platform::get_system_accent_palette() {
                theme.set_accent(accent_palette.accent.into());
                theme.set_accent_light_1(accent_palette.accent_light_1.into());
                theme.set_accent_light_2(accent_palette.accent_light_2.into());
                theme.set_accent_light_3(accent_palette.accent_light_3.into());
                theme.set_accent_dark_1(accent_palette.accent_dark_1.into());
                theme.set_accent_dark_2(accent_palette.accent_dark_2.into());
                theme.set_accent_dark_3(accent_palette.accent_dark_3.into());
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
        if type_id == TypeId::of::<dyn UiServiceDetailsPort>() {
            let port: Box<dyn Any> = Box::new(self.clone());
            Some(port)
        } else {
            None
        }
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

impl std::fmt::Debug for ServicesUiAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServicesUiAdapter").finish()
    }
}
