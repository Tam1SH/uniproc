use crate::features::tabs::state::TabsState;
#[cfg(target_os = "windows")]
use app_contracts::features::agents::WindowsAgentRuntimeEvent;
use app_contracts::features::agents::{
    AgentConnectionState, RemoteScanResult, WslAgentRuntimeEvent,
};
use app_contracts::features::navigation::NavigationProjectionChanged;
use app_contracts::features::sidebar::RequestTransition;
use app_contracts::features::tabs::{
    AvailableContextDescriptor, TabContextKey, TabContextSnapshot, TabDescriptor, TabsBinder,
    TabsPartialBinder, UiTabsBindings, UiTabsPort, UiTabsPortMsg,
};
use forsl_core::actor::event_bus::EventBus;
use forsl_core::actor::{Context, ManagedActor};
use context::page_status::{PageStatus, RouteStatusChanged};
use forsl::navigation::{RouteActivated, RouteRegistry};
use forsl_macros::handler;
use macros::actor_manifest;
use std::sync::Arc;
use tracing::{instrument, warn};

#[actor_manifest(binder = TabsBinder)]
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
        bind {
            RequestTabSwitch(String),
            RequestTabClose(String),
            RequestTabAdd(String),
        },
        @RouteActivated,
        @RouteStatusChanged,
        @NavigationProjectionChanged,
        @RemoteScanResult,
        @WslAgentRuntimeEvent,
        #[cfg(target_os = "windows")]
        @WindowsAgentRuntimeEvent
    );
    type Signals = bus!(RequestTransition, NavigationProjectionChanged);
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
        self.ui_port
            .send(UiTabsPortMsg::SetTabs(self.state.tabs().to_vec()));
        self.ui_port.send(UiTabsPortMsg::SetAvailableContexts(
            self.state.available_contexts().to_vec(),
        ));
        if let Some(active_context_key) = self.state.active_context_key() {
            self.ui_port.send(UiTabsPortMsg::SetActiveContext(
                active_context_key.clone(),
            ));
            if let Some(active_page) = self.state.active_page_for_context(active_context_key) {
                self.ui_port.send(UiTabsPortMsg::SetActivePage {
                    context_key: active_context_key.clone(),
                    route_segment: active_page.route_segment.clone(),
                });
            }
        }
    }

    fn publish_transition_if_needed(
        &self,
        transition: &crate::features::tabs::state::RouteActivation,
        ctx: &Context<Self>,
    ) {
        if let Some(previous_index) = transition.previous_index {
            if previous_index != transition.next_index {
                ctx.publish(RequestTransition {
                    from_index: previous_index as i32,
                    to_index: transition.next_index as i32,
                });
            }
        }
    }

    fn publish_state(&self, ctx: &Context<Self>) {
        ctx.publish(self.state.navigation_projection());
    }

    fn update_context_status(
        &mut self,
        context_key: &str,
        status: PageStatus,
        ctx: &Context<Self>,
    ) {
        if self.state.update_context_status(context_key, status) {
            self.sync_ui_to_state();
            self.publish_state(ctx);
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
fn switch_tab<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: RequestTabSwitch,
    ctx: &Context<TabsActor<P>>,
) {
    let context_key = TabContextKey(std::borrow::Cow::Owned(msg.0.clone()));
    if this.state.switch_to_context(&context_key) {
        this.sync_ui_to_state();
        this.publish_state(ctx);
        return;
    }

    warn!(context_key = %msg.0, "Switch failed: context not found or already active");
}

#[handler]
fn close_tab<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: RequestTabClose,
    ctx: &Context<TabsActor<P>>,
) {
    if this.state.disable_context(&msg.0) {
        this.sync_ui_to_state();
        this.publish_state(ctx);
    }
}

#[handler]
fn add_tab<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: RequestTabAdd,
    ctx: &Context<TabsActor<P>>,
) {
    if this.state.enable_context(&msg.0) {
        this.sync_ui_to_state();
        this.publish_state(ctx);
    }
}

#[handler]
fn sync_active_route<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: RouteActivated,
    ctx: &Context<TabsActor<P>>,
) {
    if let Some(transition) = this
        .state
        .activate_route(&TabContextKey(msg.uri.context_name), &msg.uri.base.segment)
    {
        this.sync_ui_to_state();
        this.publish_transition_if_needed(&transition, ctx);
        this.publish_state(ctx);
    }
}

#[handler]
fn sync_route_status<P: UiTabsPort + Clone>(this: &TabsActor<P>, msg: RouteStatusChanged) {
    let context_key = TabContextKey(std::borrow::Cow::Owned(msg.context_key.clone()));
    this.ui_port.send(UiTabsPortMsg::SetRouteStatus {
        context_key: context_key.clone(),
        route_segment: msg.route_segment.clone(),
        status: msg.status,
    });
    if let Some(err) = msg.error {
        this.ui_port.send(UiTabsPortMsg::SetRouteError {
            context_key,
            route_segment: msg.route_segment,
            msg: err,
        });
    }
}

#[handler]
fn sync_navigation_projection<P: UiTabsPort + Clone>(
    _: &TabsActor<P>,
    _: NavigationProjectionChanged,
) {
}

#[handler]
fn process_remote_scan<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: RemoteScanResult,
    ctx: &Context<TabsActor<P>>,
) {
    if this.state.apply_remote_contexts(&msg) {
        this.sync_ui_to_state();
        this.publish_state(ctx);
    }
}

#[handler]
fn sync_wsl_status<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: WslAgentRuntimeEvent,
    ctx: &Context<TabsActor<P>>,
) {
    this.update_context_status("wsl", runtime_state_to_page_status(msg.state), ctx);
}

#[cfg(target_os = "windows")]
#[handler]
fn sync_windows_status<P: UiTabsPort + Clone>(
    this: &mut TabsActor<P>,
    msg: WindowsAgentRuntimeEvent,
    ctx: &Context<TabsActor<P>>,
) {
    this.update_context_status("host", runtime_state_to_page_status(msg.state), ctx);
}
