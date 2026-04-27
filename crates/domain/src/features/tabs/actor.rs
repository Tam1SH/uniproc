use crate::features::tabs::state::TabsState;
use app_contracts::features::agents::RemoteScanResult;
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::environments::{AgentConnectionState, WslAgentRuntimeEvent};
use app_contracts::features::navigation::NavigationProjectionChanged;
use app_contracts::features::sidebar::RequestTransition;
use app_contracts::features::tabs::{
    AvailableContextDescriptor, TabContextKey, TabContextSnapshot, TabDescriptor, UiTabsPort,
};
use app_core::actor::event_bus::EventBus;
use app_core::actor::ManagedActor;
use context::page_status::{PageStatus, RouteStatusChanged};
use framework::navigation::{RouteActivated, RouteRegistry};
use macros::{actor_manifest, handler};
use std::sync::Arc;
use tracing::{instrument, warn};

#[actor_manifest]
impl<P: UiTabsPort + Clone> ManagedActor for TabsActor<P> {
    type Bus = bus!(
        @RouteActivated,
        @RouteStatusChanged,
        @RemoteScanResult,
        @WslAgentRuntimeEvent,
        #[cfg(target_os = "windows")]
        @WindowsAgentRuntimeEvent,
    );
    type Handlers = handlers!(
        RequestTabSwitch(String),
        RequestTabClose(String),
        RequestTabAdd(String),
        @RouteActivated,
        @RouteStatusChanged,
        @NavigationProjectionChanged,
        @RemoteScanResult,
        @WslAgentRuntimeEvent,
        #[cfg(target_os = "windows")]
        @WindowsAgentRuntimeEvent
    );
}

pub struct TabsActor<P: UiTabsPort + Clone> {
    ui_port: P,
    state: TabsState,
}

impl<P: UiTabsPort + Clone> TabsActor<P> {
    pub fn new(ui_port: P, contexts: Vec<TabContextSnapshot>, routes: Arc<RouteRegistry>) -> Self {
        Self {
            ui_port,
            state: TabsState::new(contexts, routes),
        }
    }

    pub fn tabs(&self) -> &[TabDescriptor] {
        self.state.tabs()
    }

    pub fn available_contexts(&self) -> &[AvailableContextDescriptor] {
        self.state.available_contexts()
    }

    pub fn active_context_key(&self) -> Option<&TabContextKey> {
        self.state.active_context_key()
    }

    pub fn sync_ui_to_state(&self) {
        self.ui_port.set_tabs(self.state.tabs().to_vec());
        self.ui_port
            .set_available_contexts(self.state.available_contexts().to_vec());
        if let Some(active_context_key) = self.state.active_context_key() {
            self.ui_port.set_active_context(active_context_key.clone());
            if let Some(active_page) = self.state.active_page_for_context(active_context_key) {
                self.ui_port.set_active_page(
                    active_context_key.clone(),
                    active_page.route_segment.clone(),
                );
            }
        }
    }

    fn publish_transition_if_needed(
        &self,
        transition: &crate::features::tabs::state::RouteActivation,
    ) {
        if let Some(previous_index) = transition.previous_index {
            if previous_index != transition.next_index {
                EventBus::publish(RequestTransition {
                    from_index: previous_index as i32,
                    to_index: transition.next_index as i32,
                });
            }
        }
    }

    fn publish_state(&self) {
        EventBus::publish(self.state.navigation_projection());
    }

    fn update_context_status(&mut self, context_key: &str, status: PageStatus) {
        if self.state.update_context_status(context_key, status) {
            self.sync_ui_to_state();
            self.publish_state();
        }
    }
}

fn runtime_state_to_page_status(state: AgentConnectionState) -> PageStatus {
    match state {
        AgentConnectionState::Connected => PageStatus::Ready,
        AgentConnectionState::Connecting => PageStatus::Loading,
        AgentConnectionState::Disconnected => PageStatus::Inactive,
        AgentConnectionState::WaitingRetry { .. } => PageStatus::Loading,
    }
}

#[handler]
fn switch_tab<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RequestTabSwitch) {
    let context_key = TabContextKey(std::borrow::Cow::Owned(msg.0.clone()));
    if this.state.switch_to_context(&context_key) {
        this.sync_ui_to_state();
        this.publish_state();
        return;
    }

    warn!(context_key = %msg.0, "Switch failed: context not found or already active");
}

#[handler]
fn close_tab<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RequestTabClose) {
    if this.state.disable_context(&msg.0) {
        this.sync_ui_to_state();
        this.publish_state();
    }
}

#[handler]
fn add_tab<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RequestTabAdd) {
    if this.state.enable_context(&msg.0) {
        this.sync_ui_to_state();
        this.publish_state();
    }
}

#[handler]
fn sync_active_route<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RouteActivated) {
    if let Some(transition) = this
        .state
        .activate_route(&TabContextKey(msg.uri.context_name), &msg.uri.base.segment)
    {
        this.sync_ui_to_state();
        this.publish_transition_if_needed(&transition);
        this.publish_state();
    }
}

#[handler]
fn sync_route_status<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RouteStatusChanged) {
    let context_key = TabContextKey(std::borrow::Cow::Owned(msg.context_key.clone()));
    this.ui_port
        .set_route_status(context_key.clone(), msg.route_segment.clone(), msg.status);
    if let Some(err) = msg.error {
        this.ui_port
            .set_route_error(context_key, msg.route_segment, err);
    }
}

#[handler]
fn sync_navigation_projection<P: UiTabsPort + Clone>(
    _: &mut TabsActor<P>,
    _: NavigationProjectionChanged,
) {
}

#[handler]
#[instrument(skip(this, msg), fields(schema_id = msg.schema_id))]
fn process_remote_scan<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: RemoteScanResult) {
    if this.state.apply_remote_contexts(&msg) {
        this.sync_ui_to_state();
        this.publish_state();
    }
}

#[handler]
#[instrument(skip(this), fields(state = ?msg.state, latency = ?msg.latency_ms))]
fn sync_wsl_status<P: UiTabsPort + Clone>(this: &mut TabsActor<P>, msg: WslAgentRuntimeEvent) {
    this.update_context_status("wsl", runtime_state_to_page_status(msg.state));
}

#[cfg(target_os = "windows")]
#[handler]
#[instrument(skip(this), fields(state = ?msg.state, latency = ?msg.latency_ms))]
fn sync_windows_status<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: WindowsAgentRuntimeEvent,
) {
    this.update_context_status("host", runtime_state_to_page_status(msg.state));
}
