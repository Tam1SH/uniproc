use crate::features::tabs::model::{
    apply_remote_contexts, build_available_contexts, build_tabs, default_enabled_context_keys,
    navigation_routes, update_context_status,
};
use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::navigation::{KnownRouteDescriptor, NavigationProjectionChanged};
use app_contracts::features::tabs::{
    AvailableContextDescriptor, TabContextKey, TabContextSnapshot, TabDescriptor,
};
use context::page_status::PageStatus;
use framework::navigation::RouteRegistry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct RouteActivation {
    pub previous_index: Option<usize>,
    pub next_index: usize,
}

pub struct TabsState {
    contexts: Vec<TabContextSnapshot>,
    pub(crate) routes: Arc<RouteRegistry>,
    tabs: Vec<TabDescriptor>,
    available_contexts: Vec<AvailableContextDescriptor>,
    active_context_key: Option<TabContextKey>,
    active_route_segment_by_context: HashMap<TabContextKey, String>,
    enabled_contexts: HashSet<TabContextKey>,
}

impl TabsState {
    pub fn new(contexts: Vec<TabContextSnapshot>, routes: Arc<RouteRegistry>) -> Self {
        let mut state = Self {
            contexts,
            routes,
            tabs: Vec::new(),
            available_contexts: Vec::new(),
            active_context_key: None,
            active_route_segment_by_context: HashMap::new(),
            enabled_contexts: HashSet::new(),
        };
        state.rebuild();
        state
    }

    pub fn tabs(&self) -> &[TabDescriptor] {
        &self.tabs
    }

    pub fn available_contexts(&self) -> &[AvailableContextDescriptor] {
        &self.available_contexts
    }

    pub fn active_context_key(&self) -> Option<&TabContextKey> {
        self.active_context_key.as_ref()
    }

    pub fn navigation_projection(&self) -> NavigationProjectionChanged {
        NavigationProjectionChanged {
            known_routes: navigation_routes(&self.tabs),
        }
    }

    pub fn known_routes(&self) -> Vec<KnownRouteDescriptor> {
        navigation_routes(&self.tabs)
    }

    pub fn active_path(&self) -> Option<String> {
        self.active_page().map(|page| page.path.clone())
    }

    pub fn switch_to_context(&mut self, context_key: &TabContextKey) -> bool {
        if self.tabs.iter().any(|tab| &tab.context_key == context_key)
            && self.active_context_key.as_ref() != Some(context_key)
        {
            self.active_context_key = Some(context_key.clone());
            return true;
        }

        false
    }

    pub fn activate_route(
        &mut self,
        context_key: &TabContextKey,
        route_segment: &str,
    ) -> Option<RouteActivation> {
        let tab = self
            .tabs
            .iter()
            .find(|tab| &tab.context_key == context_key)?;
        let next_index = tab
            .pages
            .iter()
            .position(|page| page.route_segment == route_segment)?;

        let previous_index = self.active_page_for_context(context_key).and_then(|page| {
            tab.pages
                .iter()
                .position(|candidate| candidate.route_segment == page.route_segment)
        });

        let is_same_route = previous_index == Some(next_index)
            && self.active_context_key.as_ref() == Some(context_key);
        if is_same_route {
            return None;
        }

        self.active_context_key = Some(context_key.clone());
        self.active_route_segment_by_context
            .insert(context_key.clone(), route_segment.to_string());

        Some(RouteActivation {
            previous_index,
            next_index,
        })
    }

    pub fn active_page(&self) -> Option<&app_contracts::features::tabs::TabPageDescriptor> {
        let active_context_key = self.active_context_key.as_ref()?;
        self.active_page_for_context(active_context_key)
    }

    pub fn active_page_for_context(
        &self,
        context_key: &TabContextKey,
    ) -> Option<&app_contracts::features::tabs::TabPageDescriptor> {
        let tab = self
            .tabs
            .iter()
            .find(|tab| &tab.context_key == context_key)?;
        self.active_route_segment_by_context
            .get(context_key)
            .and_then(|route_segment| {
                tab.pages
                    .iter()
                    .find(|page| page.route_segment == *route_segment)
            })
            .or_else(|| tab.pages.first())
    }

    pub fn enable_context(&mut self, context_key: &str) -> bool {
        let Some(context_key) = self
            .contexts
            .iter()
            .find(|context| context.key.0 == context_key)
            .map(|context| context.key.clone())
        else {
            return false;
        };

        if !self.enabled_contexts.insert(context_key.clone()) {
            return false;
        }

        self.rebuild();
        self.active_context_key = Some(context_key);
        true
    }

    pub fn disable_context(&mut self, context_key: &str) -> bool {
        let Some(tab) = self
            .tabs
            .iter()
            .find(|tab| tab.context_key.0 == context_key)
        else {
            return false;
        };

        if !tab.is_closable || !self.enabled_contexts.remove(&tab.context_key) {
            return false;
        }

        self.rebuild();
        true
    }

    pub fn apply_remote_contexts(&mut self, report: &RemoteScanResult) -> bool {
        if apply_remote_contexts(&mut self.contexts, report) {
            self.rebuild();
            return true;
        }

        false
    }

    pub fn update_context_status(&mut self, context_key: &str, status: PageStatus) -> bool {
        if update_context_status(&mut self.contexts, context_key, status) {
            self.rebuild();
            return true;
        }

        false
    }

    fn rebuild(&mut self) {
        let previous_active_context = self.active_context_key.clone();
        let previous_active_route_segment_by_context = self.active_route_segment_by_context.clone();

        if self.enabled_contexts.is_empty() {
            self.enabled_contexts = default_enabled_context_keys(&self.contexts)
                .into_iter()
                .collect();
        } else {
            self.enabled_contexts.retain(|context_key| {
                self.contexts
                    .iter()
                    .any(|context| &context.key == context_key)
            });
            if self.enabled_contexts.is_empty() {
                self.enabled_contexts = default_enabled_context_keys(&self.contexts)
                    .into_iter()
                    .collect();
            }
        }

        let enabled_contexts: Vec<_> = self
            .contexts
            .iter()
            .filter(|context| self.enabled_contexts.contains(&context.key))
            .cloned()
            .collect();

        self.tabs = build_tabs(&enabled_contexts, &self.routes);
        self.available_contexts = build_available_contexts(&self.contexts, &self.enabled_contexts);
        self.active_context_key = previous_active_context
            .filter(|context_key| self.tabs.iter().any(|tab| &tab.context_key == context_key))
            .or_else(|| self.tabs.first().map(|tab| tab.context_key.clone()));

        self.active_route_segment_by_context.clear();
        for tab in &self.tabs {
            let preserved_route = previous_active_route_segment_by_context
                .get(&tab.context_key)
                .filter(|route_segment| {
                    tab.pages
                        .iter()
                        .any(|page| page.route_segment == route_segment.as_str())
                })
                .cloned()
                .or_else(|| tab.pages.first().map(|page| page.route_segment.clone()));

            if let Some(route_segment) = preserved_route {
                self.active_route_segment_by_context
                    .insert(tab.context_key.clone(), route_segment);
            }
        }
    }
}
