use crate::features::navigation::model::ActiveRoute;
use crate::features::navigation::state::{NavigationState, RouteSwitchPlan};
use app_contracts::features::navigation::{KnownRouteDescriptor, NavigationProjectionChanged};
use app_core::actor::event_bus::EventBus;
use app_core::actor::ManagedActor;
use framework::navigation::{RouteActivated, RouteDeactivated, RouteRegistry};
use macros::{actor_manifest, handler};
use std::borrow::Cow;
use std::sync::Arc;
use tracing::{info, warn};

#[actor_manifest]
impl ManagedActor for NavigationActor {
    type Bus = bus!();
    type Handlers = handlers!(Push(String));
}

pub struct NavigationActor {
    state: NavigationState,
    window_id: usize,
}

impl NavigationActor {
    pub fn new(registry: Arc<RouteRegistry>, window_id: usize) -> Self {
        Self {
            state: NavigationState::new(registry),
            window_id,
        }
    }

    fn apply_switch_plan(&mut self, switch_plan: RouteSwitchPlan) {
        info!(
            "Switching route. previous uri: {}, current uri: {}",
            switch_plan.previous_uri.clone().unwrap_or_default(),
            switch_plan.uri
        );

        if let Some(uri) = switch_plan.previous_uri {
            EventBus::publish(RouteDeactivated {
                window_id: self.window_id,
                uri,
            });
        }

        EventBus::publish(RouteActivated {
            window_id: self.window_id,
            uri: switch_plan.uri,
        });
    }
}

#[handler]
fn push(this: &mut NavigationActor, msg: Push) {
    let Some(switch_plan) = this.state.push(&msg.0) else {
        if this.state.active_route_segment() == Some(Cow::from(msg.0.clone()).into()) {
            if this.state.active_route().is_some() {
                return;
            }
        }
        warn!(route_segment = %msg.0, "Switch failed: route unavailable");
        return;
    };

    this.apply_switch_plan(switch_plan);
}
