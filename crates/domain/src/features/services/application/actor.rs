use crate::features::services::view::ServiceTable;
use app_contracts::features::agents::{WindowsActionRequest, WindowsActionResponse};
use app_contracts::features::navigation::{tab_ids, PageActivated};
use app_contracts::features::services::{
    ServiceActionKind, ServiceEntryVm, ServiceSnapshot, UiServiceDetailsPort,
    UiServicesPort, PROPERTIES_DIALOG_KEY,
};
use app_contracts::features::windows_manager::OpenedWindow;
use app_core::actor::event_bus::EventBus;
use app_core::feature::FeatureContextState;
use app_core::messages;
use app_core::trace::current_or_new_correlation_uuid;
use context::native_windows::slint_factory::{OpenWindow, SlintWindowRegistry, WindowRegistry};
use context::page_status::{PageId, PageStatus, PageStatusChanged, PageStatusRegistry};
use macros::handler;
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

pub struct ServiceActor<P: UiServicesPort> {
    pub page_id: PageId,
    pub table: ServiceTable,
    pub registry: Arc<SlintWindowRegistry>,
    pub ui_port: P,
    pub page_status: Arc<PageStatusRegistry>,
    pub is_active: bool,
    pub pending: HashSet<Uuid>,
    pub ctx_state: FeatureContextState,
}

impl<P: UiServicesPort> ServiceActor<P> {
    fn push_batch(&self) {
        let b = self.table.batch();
        self.ui_port
            .set_service_rows_window(b.total_rows, b.start, b.rows);
    }
}

#[handler]
fn service_snapshot<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ServiceSnapshot) {
    if !this.is_active {
        return;
    }

    this.ui_port.set_total_services_count(msg.services.len());
    this.table.update_data(msg.services);
    this.ui_port.set_column_widths(this.table.column_widths());

    this.page_status.report_page(PageStatusChanged {
        tab_id: tab_ids::MAIN,
        page_id: this.page_id,
        status: PageStatus::Ready,
        error: None,
    });

    this.push_batch();
}

#[handler]
fn service_action<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ServiceAction) {
    let id = current_or_new_correlation_uuid();
    let cmd = match msg.kind {
        ServiceActionKind::Start => ServiceCommand::Start { name: msg.name },
        ServiceActionKind::Stop => ServiceCommand::Stop { name: msg.name },
        ServiceActionKind::Restart => ServiceCommand::Restart { name: msg.name },
        ServiceActionKind::Pause => ServiceCommand::Pause { name: msg.name },
        ServiceActionKind::Resume => ServiceCommand::Resume { name: msg.name },
    };

    this.pending.insert(id);
    EventBus::publish(WindowsActionRequest::new(
        id,
        WindowsRequest::ServiceCommand(cmd),
    ));
}

#[handler]
fn on_action_response<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: WindowsActionResponse) {
    this.pending.remove(&msg.correlation_id);
}

#[handler]
fn resize_service_column<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ResizeCol) {
    let _ = this
        .table
        .resize_column(msg.id.to_string(), msg.width as u64);
    this.ui_port.set_column_widths(this.table.column_widths());
}

#[handler]
fn open_external_services<P: UiServicesPort>(_: &mut ServiceActor<P>, _: OpenServices) {
    // #[cfg(target_os = "windows")]
    // let _ = std::process::Command::new("mmc.exe")
    //     .arg("services.msc")
    //     .spawn();
}

#[handler]
fn activate_page<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: PageActivated) {
    this.is_active = msg.page_id == this.page_id;
}

#[handler]
fn sort_services<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: Sort) {
    let s = &mut this.table.view.flow.sort;
    if s.field_id.as_ref() == Some(&msg.0) {
        s.descending = !s.descending;
    } else {
        s.field_id = Some(msg.0.clone());
        s.descending = false;
    }

    this.ui_port.set_current_sort_descending(s.descending);
    this.ui_port.set_current_sort(msg.0);

    this.table.refresh();
    this.push_batch();
}

#[handler]
fn change_viewport<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ViewportChanged) {
    this.table.view.rows.set_viewport(msg.start, msg.count);
    this.push_batch();
}

#[handler]
fn select_service<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: SelectedService) {
    if let Some(dto) = this.table.get_by_name(msg.0.as_str()) {
        match dto.status.as_str() {
            "Running" => this.ui_port.set_active_buttons(false, true, true),
            "Stopped" => this.ui_port.set_active_buttons(true, false, false),
            _ => {}
        }

        this.ui_port
            .set_selected_service_details(dto.clone().into());
    }

    this.table.select(msg.0.clone(), msg.1);
}

#[handler]
fn open_properties<P: UiServicesPort>(_: &mut ServiceActor<P>, msg: OpenPropertiesWindow) {
    EventBus::publish(OpenWindow {
        key: msg.0.name.to_string(),
        template: PROPERTIES_DIALOG_KEY.to_string(),
        data: Arc::new(msg.0),
    })
}

#[handler]
fn on_window_opened<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: OpenedWindow) {
    let Some(window) = this.registry.get_window(&msg.key) else {
        return;
    };
    let Some(ui_port) = window.get_port::<dyn UiServiceDetailsPort>() else {
        return;
    };

    let dto = msg
        .data
        .downcast::<ServiceEntryVm>()
        .expect("ServiceEntryVm is of wrong type");

    match dto.status.as_str() {
        "Running" => ui_port.set_active_buttons(false, true, true),
        "Stopped" => ui_port.set_active_buttons(true, false, false),
        _ => {}
    }

    ui_port.set_selected_service_details(dto.as_ref().clone().into());
}
