use crate::features::services::view::ServiceTable;
use app_contracts::features::agents::{WindowsActionRequest, WindowsActionResponse};
use app_contracts::features::navigation::{tab_ids, PageActivated};
use app_contracts::features::services::{
    ServiceActionKind, ServiceDetailsPort, ServiceEntryVm, ServiceSnapshot, ServicesUiPort,
    PROPERTIES_DIALOG_KEY,
};
use app_contracts::features::windows_manager::OpenedWindow;
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;
use app_core::messages;
use app_core::trace::current_or_new_correlation_uuid;
use context::native_windows::slint_factory::{OpenWindow, SlintWindowRegistry, WindowRegistry};
use context::page_status::{PageId, PageStatus, PageStatusChanged, PageStatusRegistry};
use slint::SharedString;
use std::collections::HashSet;
use std::sync::Arc;
use uniproc_protocol::{ServiceCommand, WindowsRequest};
use uuid::Uuid;

messages! {
    ServiceAction { name: String, kind: ServiceActionKind },
    Sort(SharedString),
    ViewportChanged { start: usize, count: usize },
    ResizeCol { id: SharedString, width: f32 },
    OpenServices,
    SelectedService(SharedString, usize),
    OpenPropertiesWindow(ServiceEntryVm),
}

pub struct ServiceActor<P: ServicesUiPort> {
    pub page_id: PageId,
    pub table: ServiceTable,
    pub registry: Arc<SlintWindowRegistry>,
    pub ui_port: P,
    pub page_status: Arc<PageStatusRegistry>,
    pub is_active: bool,
    pub pending: HashSet<Uuid>,
}

impl<P: ServicesUiPort, T: Window> Handler<ServiceSnapshot, T> for ServiceActor<P> {
    fn handle(&mut self, m: ServiceSnapshot, _: &Context<Self, T>) {
        self.ui_port.set_total_services_count(m.services.len());

        self.table.update_data(m.services);

        self.ui_port.set_column_widths(self.table.column_widths());

        self.page_status.report_page(PageStatusChanged {
            tab_id: tab_ids::MAIN,
            page_id: self.page_id,
            status: PageStatus::Ready,
            error: None,
        });
        let b = self.table.batch();
        self.ui_port
            .set_service_rows_window(b.total_rows, b.start, b.rows);
    }
}

impl<P: ServicesUiPort, T: Window> Handler<ServiceAction, T> for ServiceActor<P> {
    fn handle(&mut self, m: ServiceAction, _: &Context<Self, T>) {
        let id = current_or_new_correlation_uuid();
        let cmd = match m.kind {
            ServiceActionKind::Start => ServiceCommand::Start { name: m.name },
            ServiceActionKind::Stop => ServiceCommand::Stop { name: m.name },
            ServiceActionKind::Restart => ServiceCommand::Restart { name: m.name },
            ServiceActionKind::Pause => ServiceCommand::Pause { name: m.name },
            ServiceActionKind::Resume => ServiceCommand::Resume { name: m.name },
        };
        self.pending.insert(id);
        EventBus::publish(WindowsActionRequest::new(
            id,
            WindowsRequest::ServiceCommand(cmd),
        ));
    }
}

impl<P: ServicesUiPort, T: Window> Handler<WindowsActionResponse, T> for ServiceActor<P> {
    fn handle(&mut self, m: WindowsActionResponse, _: &Context<Self, T>) {
        self.pending.remove(&m.correlation_id);
    }
}

impl<P: ServicesUiPort, T: Window> Handler<ResizeCol, T> for ServiceActor<P> {
    fn handle(&mut self, m: ResizeCol, _: &Context<Self, T>) {
        let _ = self.table.resize_column(m.id.to_string(), m.width as u64);
        self.ui_port.set_column_widths(self.table.column_widths());
    }
}

impl<P: ServicesUiPort, T: Window> Handler<OpenServices, T> for ServiceActor<P> {
    fn handle(&mut self, _: OpenServices, _: &Context<Self, T>) {
        // info!("lol");
        // #[cfg(target_os = "windows")]
        // let _ = dbg!(
        //     std::process::Command::new("mmc.exe")
        //         .arg("services.msc")
        //         .spawn()
        // );
    }
}

impl<P: ServicesUiPort, T: Window> Handler<PageActivated, T> for ServiceActor<P> {
    fn handle(&mut self, m: PageActivated, _: &Context<Self, T>) {
        self.is_active = m.page_id == self.page_id;
    }
}

impl<P: ServicesUiPort, T: Window> Handler<Sort, T> for ServiceActor<P> {
    fn handle(&mut self, m: Sort, _: &Context<Self, T>) {
        let s = &mut self.table.view.flow.sort;
        if s.field_id.as_ref() == Some(&m.0) {
            s.descending = !s.descending;
        } else {
            s.field_id = Some(m.0.clone());
            s.descending = false;
        }
        self.ui_port.set_sort_state(m.0, s.descending);
        self.table.refresh();
        let b = self.table.batch();
        self.ui_port
            .set_service_rows_window(b.total_rows, b.start, b.rows);
    }
}

impl<P: ServicesUiPort, T: Window> Handler<ViewportChanged, T> for ServiceActor<P> {
    fn handle(&mut self, m: ViewportChanged, _: &Context<Self, T>) {
        self.table.view.rows.set_viewport(m.start, m.count);
        let b = self.table.batch();
        self.ui_port
            .set_service_rows_window(b.total_rows, b.start, b.rows);
    }
}

impl<P: ServicesUiPort, T: Window> Handler<SelectedService, T> for ServiceActor<P> {
    fn handle(&mut self, m: SelectedService, _: &Context<Self, T>) {
        if let Some(dto) = self.table.get_by_name(m.0.as_str()) {
            match dto.status.as_str() {
                "Running" => {
                    self.ui_port.set_active_buttons(false, true, true);
                }
                "Stopped" => {
                    self.ui_port.set_active_buttons(true, false, false);
                }
                _ => {}
            }

            self.ui_port
                .set_selected_service_details(dto.clone().into());
        }

        self.table.select(m.0.clone(), m.1);
    }
}

impl<P: ServicesUiPort, T: Window> Handler<OpenPropertiesWindow, T> for ServiceActor<P> {
    fn handle(&mut self, m: OpenPropertiesWindow, _: &Context<Self, T>) {
        EventBus::publish(OpenWindow {
            key: m.0.name.to_string(),
            template: PROPERTIES_DIALOG_KEY.to_string(),
            data: Arc::new(m.0),
        })
    }
}

impl<P: ServicesUiPort, T: Window> Handler<OpenedWindow, T> for ServiceActor<P> {
    fn handle(&mut self, m: OpenedWindow, _: &Context<Self, T>) {
        if let Some(window) = self.registry.get_window(&m.key) {
            if let Some(ui_port) = window.get_port::<dyn ServiceDetailsPort>() {
                let dto = m
                    .data
                    .downcast::<ServiceEntryVm>()
                    .expect("ServiceEntryVm is of wrong type");

                match dto.status.as_str() {
                    "Running" => {
                        ui_port.set_active_buttons(false, true, true);
                    }
                    "Stopped" => {
                        ui_port.set_active_buttons(true, false, false);
                    }
                    _ => {}
                }

                ui_port.set_selected_service_details(dto.as_ref().clone().into());
            }
        }
    }
}
