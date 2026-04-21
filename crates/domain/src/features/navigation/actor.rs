use crate::features::navigation::settings::NavigationSettings;
use crate::features::navigation::state::NavigationState;
use app_contracts::features::agents::RemoteScanResult;
#[cfg(target_os = "windows")]
use app_contracts::features::environments::WindowsAgentRuntimeEvent;
use app_contracts::features::environments::{AgentConnectionState, WslAgentRuntimeEvent};
use app_contracts::features::navigation::{
    AvailableContextDescriptor, NavigationContextsChanged, PageActivated, TabActivated,
    TabContextSnapshot, UiNavigationPort,
};
use app_contracts::features::sidebar::RequestTransition;
use app_core::actor::event_bus::EventBus;
use app_core::messages;
use app_core::trace::{current_meta, install_current_meta};
use context::page_status::{
    PageId, PageStatus, PageStatusChanged, PageStatusRegistry, TabId, TabStatusChanged,
};
use macros::handler;
use std::sync::Arc;
use tracing::instrument;
use tracing::{debug, info, warn};

messages! {
    RequestPageSwitch(TabId, PageId),
    RequestTabSwitch(TabId),
    RequestTabClose(TabId),
    RequestTabAdd(String),
}

pub struct NavigationActor<P: UiNavigationPort + Clone> {
    ui_port: P,
    registry: Arc<PageStatusRegistry>,
    state: NavigationState,
}

impl<P: UiNavigationPort + Clone> NavigationActor<P> {
    pub fn new(
        ui_port: P,
        registry: Arc<PageStatusRegistry>,
        contexts: Vec<TabContextSnapshot>,
        _settings: &NavigationSettings,
    ) -> Self {
        Self {
            ui_port,
            registry,
            state: NavigationState::new(contexts),
        }
    }

    pub fn tabs(&self) -> &[app_contracts::features::navigation::TabDescriptor] {
        self.state.tabs()
    }

    pub fn available_contexts(&self) -> &[AvailableContextDescriptor] {
        self.state.available_contexts()
    }

    pub fn resolve_initial_route(&self, candidate: (TabId, PageId)) -> Option<(TabId, PageId)> {
        self.state.resolve_initial_route(candidate)
    }

    fn sync_ui_to_state(&self) {
        self.ui_port.set_navigation_tree(self.state.tabs().to_vec());
        self.ui_port
            .set_available_contexts(self.state.available_contexts().to_vec());

        if let Some((active_tab_id, active_page_id)) = self.state.active_route() {
            self.ui_port.set_active_tab(active_tab_id);
            self.ui_port.set_active_page(active_tab_id, active_page_id);
        }
    }

    fn update_context_status(&mut self, context_key: &str, status: PageStatus) {
        if self.state.update_context_status(context_key, status) {
            self.sync_ui_to_state();
        }
    }

    #[instrument(skip(self), fields(tab_id = ?tab_id, page_id = ?page_id))]
    fn perform_page_switch(&mut self, tab_id: TabId, page_id: PageId) {
        let Some(switch_plan) = self.state.switch_to_page(tab_id, page_id) else {
            warn!("Switch failed or skipped: target route unavailable");
            return;
        };

        info!(
            context_key = switch_plan.context_key.0.as_ref(),
            from_tab = ?switch_plan.previous_tab_id,
            to_tab = ?switch_plan.tab_id,
            from_page = ?switch_plan.previous_page_id,
            to_page = ?switch_plan.page_id,
            "Switching page"
        );

        EventBus::publish(TabActivated {
            tab_id: switch_plan.tab_id,
        });

        EventBus::publish(PageActivated {
            tab_id: switch_plan.tab_id,
            page_id: switch_plan.page_id,
        });

        let page_state = self
            .registry
            .get_page_state(switch_plan.tab_id, switch_plan.page_id);
        self.ui_port
            .set_page_status(switch_plan.tab_id, switch_plan.page_id, page_state.status);
        if switch_plan.from_index != -1 {
            debug!(
                from_index = switch_plan.from_index,
                to_index = switch_plan.to_index,
                "Switching page within active context"
            );
            EventBus::publish(RequestTransition {
                from_index: switch_plan.from_index,
                to_index: switch_plan.to_index,
            });
        }

        let _meta_guard = current_meta().map(install_current_meta);
        self.ui_port.set_active_tab(switch_plan.tab_id);
        self.ui_port
            .set_active_page(switch_plan.tab_id, switch_plan.page_id);
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
fn switch_page<P: UiNavigationPort + Clone>(this: &mut NavigationActor<P>, msg: RequestPageSwitch) {
    this.perform_page_switch(msg.0, msg.1);
}

#[handler]
#[instrument(skip(this))]
fn switch_tab<P: UiNavigationPort + Clone>(this: &mut NavigationActor<P>, msg: RequestTabSwitch) {
    let tab_id = msg.0;
    let Some(page_id) = this.state.page_for_tab(tab_id) else {
        warn!(?tab_id, "Switch failed: no page available for tab");
        return;
    };

    this.perform_page_switch(tab_id, page_id);
}

#[handler]
#[instrument(skip(this, msg), fields(context_count = msg.contexts.len()))]
fn update_navigation_contexts<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: NavigationContextsChanged,
) {
    this.state.replace_contexts(msg.contexts);
    this.sync_ui_to_state();
}

#[handler]
#[instrument(skip(this, msg), fields(schema_id = msg.schema_id))]
fn process_remote_scan<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: RemoteScanResult,
) {
    if this.state.apply_remote_contexts(&msg) {
        this.sync_ui_to_state();
    }
}

#[handler]
#[instrument(skip(this), fields(state = ?msg.state, latency = ?msg.latency_ms))]
fn sync_wsl_status<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: WslAgentRuntimeEvent,
) {
    this.update_context_status("wsl", runtime_state_to_page_status(msg.state));
}

#[cfg(target_os = "windows")]
#[handler]
#[instrument(skip(this), fields(state = ?msg.state, latency = ?msg.latency_ms))]
fn sync_windows_status<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: WindowsAgentRuntimeEvent,
) {
    this.update_context_status("host/windows", runtime_state_to_page_status(msg.state));
}

#[handler]
#[instrument(skip(this), fields(tab_id = ?msg.tab_id, page_id = ?msg.page_id, status = ?msg.status))]
fn update_page_status<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: PageStatusChanged,
) {
    this.ui_port
        .set_page_status(msg.tab_id, msg.page_id, msg.status);
    if let Some(err) = msg.error {
        warn!(error = %err, "Page error reported");
        this.ui_port.set_page_error(msg.tab_id, msg.page_id, err);
    }
}

#[handler]
#[instrument(skip(this), fields(tab_id = ?msg.tab_id, status = ?msg.status))]
fn update_tab_status<P: UiNavigationPort + Clone>(
    this: &mut NavigationActor<P>,
    msg: TabStatusChanged,
) {
    this.ui_port.set_tab_status(msg.tab_id, msg.status);
    if let Some(err) = msg.error {
        warn!(error = %err, "Tab error reported");
        this.ui_port.set_tab_error(msg.tab_id, err);
    }
}

#[handler]
fn close_tab<P: UiNavigationPort + Clone>(this: &mut NavigationActor<P>, msg: RequestTabClose) {
    if this.state.disable_context(msg.0) {
        this.sync_ui_to_state();
    }
}

#[handler]
fn add_tab<P: UiNavigationPort + Clone>(this: &mut NavigationActor<P>, msg: RequestTabAdd) {
    if this.state.enable_context(&msg.0) {
        this.sync_ui_to_state();
    }
}
