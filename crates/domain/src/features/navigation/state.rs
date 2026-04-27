use crate::features::navigation::model::ActiveRoute;
use framework::navigation::{Route, RouteRegistry};
use framework::uri::{AppUri, SegmentName};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RouteSwitchPlan {
    pub previous_uri: Option<AppUri>,
    pub uri: AppUri,
}

pub struct NavigationState {
    registry: Arc<RouteRegistry>,
    active_path: Option<AppUri>,
}

impl NavigationState {
    pub fn new(registry: Arc<RouteRegistry>) -> Self {
        Self {
            registry,
            active_path: None,
        }
    }

    pub fn active_route(&self) -> Option<ActiveRoute> {
        let active_page = self.active_known_route()?;
        let context = self.active_path.as_ref()?;
        let uri = active_page.uri.with_context(context.context_name.clone());

        Some(ActiveRoute { uri })
    }

    pub fn active_route_segment(&self) -> Option<SegmentName> {
        self.active_known_route().map(|route| route.uri.segment)
    }

    pub fn push(&mut self, segment: &str) -> Option<RouteSwitchPlan> {
        let known_routes = self.registry.all();

        let target_route = known_routes
            .iter()
            .find(|route| *route.uri.segment == segment)
            .or_else(|| {
                known_routes
                    .iter()
                    .find(|route| *route.uri.segment == segment)
            })?
            .clone();

        if self.active_path.as_ref().map(|p| &p.base) == Some(&target_route.uri) {
            return None;
        }

        let context = self
            .active_path
            .as_ref()
            .map_or(Cow::Borrowed("host"), |p| p.context_name.clone());

        let previous_uri = self.active_path.clone();
        self.active_path = Some(target_route.uri.with_context(context));
        Some(RouteSwitchPlan {
            previous_uri,
            uri: self.active_path.as_ref().unwrap().clone(),
        })
    }
}

impl NavigationState {
    fn active_known_route(&self) -> Option<Route> {
        let active_path = self.active_path.as_ref()?;
        let known_routes = self.registry.all();

        known_routes
            .iter()
            .find(|route| route.uri == active_path.base)
            .cloned()
    }
}
