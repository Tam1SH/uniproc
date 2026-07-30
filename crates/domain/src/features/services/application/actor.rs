use crate::features::services::ServicesBinder;
use crate::features::services::UiServicesBindings;
use crate::features::services::application::snapshot_actor::ActiveStatus;
use crate::features::services::view::ServiceTable;
use app_contracts::features::agents::{WindowsActionRequest, WindowsActionResponse};
use app_contracts::features::services::ServicesPartialBinder;
use app_contracts::features::services::{
    PROPERTIES_DIALOG_KEY, ServiceActionKind, ServiceEntryVm, ServiceSnapshot,
    UiServiceDetailsPort, UiServiceDetailsPortMsg, UiServicesPort, UiServicesPortMsg,
};
use app_contracts::features::windows_manager::OpenedWindow;
use forsl_core::actor::Context;
use forsl_core::actor::ManagedActor;
use forsl_core::trace::current_or_new_correlation_uuid;
use context::page_status::{PageStatus, RouteStatusChanged, RouteStatusRegistry};
use forsl::feature::{Events, FeatureComponent, FeatureContextState};
use forsl::native_windows::slint_factory::{OpenWindow, SlintWindowRegistry, WindowRegistry};
use forsl::uri::AppUri;
use forsl_macros::handler;
use macros::actor_manifest;
use slint::SharedString;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use uniproc_protocol::{ServiceCommand, WindowsRequest};
use uuid::Uuid;

#[actor_manifest(binder = ServicesBinder)]
impl<P: UiServicesPort> ManagedActor for ServiceActor<P> {
    type Bus = Events<bus!(ServiceSnapshot, WindowsActionResponse, OpenedWindow)>;
    type Handlers = handlers!(
        @ServiceSnapshot,
        @WindowsActionResponse,
        @OpenedWindow,
        bind {
            ServiceAction {
                name: SharedString,
                kind: ServiceActionKind
            },
            SortBy(SharedString),
            RowsViewportChanged {
                start: i32,
                count: i32
            },
            ColumnResized {
                id: SharedString,
                width: f32
            },
            SelectService(SharedString, i32),
            OpenPropertiesWindow(ServiceEntryVm)
        },
    );
    type Signals = bus!(ActiveStatus, OpenWindow, WindowsActionRequest);
}

pub struct ServiceActor<P: UiServicesPort> {
    pub table: ServiceTable,
    pub registry: Arc<SlintWindowRegistry>,
    pub ui_port: P,
    pub route_status: Arc<RouteStatusRegistry>,
    pub is_active: bool,
    pub active_context_key: Cow<'static, str>,
    pub pending: HashSet<Uuid>,
    pub ctx_state: FeatureContextState,
}

impl<P: UiServicesPort> FeatureComponent for ServiceActor<P> {
    fn context_state(&mut self) -> &mut FeatureContextState {
        &mut self.ctx_state
    }

    fn on_activated(&mut self, uri: &AppUri, ctx: &Context<Self>) {
        self.is_active = true;

        ctx.publish(ActiveStatus(true));
        self.active_context_key = uri.context_name.clone();
    }

    fn on_deactivated(&mut self, _: &AppUri, ctx: &Context<Self>) {
        self.is_active = false;
        ctx.publish(ActiveStatus(false));
    }
}

impl<P: UiServicesPort> ServiceActor<P> {
    fn push_batch(&self) {
        let b = self.table.batch();
        UiServicesPort::send(
            &self.ui_port,
            UiServicesPortMsg::SetServiceRowsWindow {
                total_rows: b.total_rows,
                start: b.start,
                rows: b.rows.to_vec(),
            },
        );
    }
}

#[handler]
fn service_snapshot<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ServiceSnapshot) {
    if !this.is_active {
        return;
    }

    UiServicesPort::send(
        &this.ui_port,
        UiServicesPortMsg::SetTotalServicesCount(msg.services.len()),
    );
    this.table.update_data(msg.services);
    UiServicesPort::send(
        &this.ui_port,
        UiServicesPortMsg::SetColumnWidths(this.table.column_widths()),
    );

    this.route_status.report_route(RouteStatusChanged {
        context_key: this.active_context_key.to_string(),
        route_segment: "services".into(),
        status: PageStatus::Ready,
        error: None,
    });

    this.push_batch();
}

#[handler]
fn service_action<P: UiServicesPort>(
    this: &mut ServiceActor<P>,
    ServiceAction { name, kind }: ServiceAction,
    ctx: &Context<ServiceActor<P>>,
) {
    let name = name.to_string();
    let id = current_or_new_correlation_uuid();
    let cmd = match kind {
        ServiceActionKind::Start => ServiceCommand::Start { name },
        ServiceActionKind::Stop => ServiceCommand::Stop { name },
        ServiceActionKind::Restart => ServiceCommand::Restart { name },
        ServiceActionKind::Pause => ServiceCommand::Pause { name },
        ServiceActionKind::Resume => ServiceCommand::Resume { name },
    };

    this.pending.insert(id);
    ctx.publish(WindowsActionRequest::new(
        id,
        WindowsRequest::ServiceCommand(cmd),
    ));
}

#[handler]
fn on_action_response<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: WindowsActionResponse) {
    this.pending.remove(&msg.correlation_id);
}

#[handler]
fn resize_service_column<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: ColumnResized) {
    let _ = this
        .table
        .resize_column(msg.id.to_string(), msg.width as u64);
    UiServicesPort::send(
        &this.ui_port,
        UiServicesPortMsg::SetColumnWidths(this.table.column_widths()),
    );
}

#[handler]
fn sort_services<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: SortBy) {
    let s = &mut this.table.view.flow.sort;
    if s.field_id.as_ref() == Some(&msg.0) {
        s.descending = !s.descending;
    } else {
        s.field_id = Some(msg.0.clone());
        s.descending = false;
    }

    UiServicesPort::send(
        &this.ui_port,
        UiServicesPortMsg::SetCurrentSortDescending(s.descending),
    );
    UiServicesPort::send(&this.ui_port, UiServicesPortMsg::SetCurrentSort(msg.0));

    this.table.refresh();
    this.push_batch();
}

#[handler]
fn change_viewport<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: RowsViewportChanged) {
    this.table
        .view
        .rows
        .set_viewport(msg.start as usize, msg.count as usize);
    this.push_batch();
}

#[handler]
fn select_service<P: UiServicesPort>(this: &mut ServiceActor<P>, msg: SelectService) {
    if let Some(dto) = this.table.get_by_name(msg.0.as_str()) {
        let active_buttons = match dto.status.as_str() {
            "Running" => Some((false, true, true)),
            "Stopped" => Some((true, false, false)),
            _ => None,
        };
        if let Some((start_button_active, stop_button_active, restart_button_active)) =
            active_buttons
        {
            UiServiceDetailsPort::send(
                &this.ui_port,
                UiServiceDetailsPortMsg::SetActiveButtons {
                    start_button_active,
                    stop_button_active,
                    restart_button_active,
                },
            );
        }

        UiServiceDetailsPort::send(
            &this.ui_port,
            UiServiceDetailsPortMsg::SetSelectedServiceDetails(dto.clone().into()),
        );
    }

    this.table.select(msg.0.clone(), msg.1 as usize);
}

#[handler]
fn open_properties<P: UiServicesPort>(
    _: &ServiceActor<P>,
    msg: OpenPropertiesWindow,
    ctx: &Context<ServiceActor<P>>,
) {
    ctx.publish(OpenWindow {
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

    let active_buttons = match dto.status.as_str() {
        "Running" => Some((false, true, true)),
        "Stopped" => Some((true, false, false)),
        _ => None,
    };
    if let Some((start_button_active, stop_button_active, restart_button_active)) =
        active_buttons
    {
        ui_port.send(UiServiceDetailsPortMsg::SetActiveButtons {
            start_button_active,
            stop_button_active,
            restart_button_active,
        });
    }

    ui_port.send(UiServiceDetailsPortMsg::SetSelectedServiceDetails(
        dto.as_ref().clone().into(),
    ));
}
