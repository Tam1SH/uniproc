use crate::features::navigation::model::{
    apply_remote_contexts, build_tabs, default_enabled_context_keys, update_context_status,
};
use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::navigation::{
    AvailableContextDescriptor, TabContextKey, TabContextSnapshot, TabDescriptor,
};
use context::page_status::{PageId, PageStatus, TabId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct PageSwitchPlan {
    pub tab_id: TabId,
    pub page_id: PageId,
    pub context_key: TabContextKey,
    pub from_index: i32,
    pub to_index: i32,
    pub previous_tab_id: Option<TabId>,
    pub previous_page_id: Option<PageId>,
}

pub struct NavigationState {
    contexts: Vec<TabContextSnapshot>,
    tabs: Vec<TabDescriptor>,
    available_contexts: Vec<AvailableContextDescriptor>,
    active_context_key: Option<TabContextKey>,
    active_page_by_context: HashMap<TabContextKey, PageId>,
    enabled_contexts: HashSet<TabContextKey>,
}

impl NavigationState {
    pub fn new(contexts: Vec<TabContextSnapshot>) -> Self {
        let mut state = Self {
            contexts,
            tabs: Vec::new(),
            available_contexts: Vec::new(),
            active_context_key: None,
            active_page_by_context: HashMap::new(),
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

    pub fn resolve_initial_route(&self, candidate: (TabId, PageId)) -> Option<(TabId, PageId)> {
        self.tabs
            .iter()
            .find(|tab| tab.id == candidate.0)
            .and_then(|tab| {
                tab.pages
                    .iter()
                    .find(|page| page.id == candidate.1)
                    .map(|page| (tab.id, page.id))
            })
            .or_else(|| self.active_route())
    }

    pub fn active_route(&self) -> Option<(TabId, PageId)> {
        let active_tab_id = self.active_tab_id()?;
        let active_page_id = self.page_for_tab(active_tab_id)?;
        Some((active_tab_id, active_page_id))
    }

    pub fn active_tab_id(&self) -> Option<TabId> {
        let active_context_key = self.active_context_key.as_ref()?;
        self.tabs
            .iter()
            .find(|tab| &tab.context_key == active_context_key)
            .map(|tab| tab.id)
    }

    pub fn page_for_tab(&self, tab_id: TabId) -> Option<PageId> {
        let tab = self.tabs.iter().find(|tab| tab.id == tab_id)?;

        self.active_page_by_context
            .get(&tab.context_key)
            .copied()
            .or_else(|| tab.pages.first().map(|page| page.id))
    }

    pub fn switch_to_page(&mut self, tab_id: TabId, page_id: PageId) -> Option<PageSwitchPlan> {
        let target_tab = self.tabs.iter().find(|tab| tab.id == tab_id)?;
        let target_page_index = target_tab.pages.iter().position(|page| page.id == page_id)? as i32;
        let target_context_key = target_tab.context_key.clone();

        let previous_tab_id = self.active_tab_id();
        let previous_page_id = previous_tab_id.and_then(|tab_id| self.page_for_tab(tab_id));

        if previous_tab_id == Some(tab_id) && previous_page_id == Some(page_id) {
            return None;
        }

        let from_index = if self.active_context_key.as_ref() == Some(&target_context_key) {
            previous_page_id
                .and_then(|current_page_id| {
                    target_tab
                        .pages
                        .iter()
                        .position(|page| page.id == current_page_id)
                        .map(|idx| idx as i32)
                })
                .unwrap_or(0)
        } else {
            -1
        };

        self.active_page_by_context
            .insert(target_context_key.clone(), page_id);
        self.active_context_key = Some(target_context_key.clone());

        Some(PageSwitchPlan {
            tab_id,
            page_id,
            context_key: target_context_key,
            from_index,
            to_index: target_page_index,
            previous_tab_id,
            previous_page_id,
        })
    }

    pub fn replace_contexts(&mut self, contexts: Vec<TabContextSnapshot>) {
        self.contexts = contexts;
        self.rebuild();
    }

    pub fn apply_remote_contexts(&mut self, report: &RemoteScanResult) -> bool {
        if apply_remote_contexts(&mut self.contexts, report) {
            self.rebuild();
            return true;
        }

        false
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

    pub fn disable_context(&mut self, tab_id: TabId) -> bool {
        let Some(tab) = self.tabs.iter().find(|tab| tab.id == tab_id) else {
            return false;
        };

        if !tab.is_closable || !self.enabled_contexts.remove(&tab.context_key) {
            return false;
        }

        self.rebuild();
        true
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
        let previous_active_page_by_context = self.active_page_by_context.clone();

        if self.enabled_contexts.is_empty() {
            self.enabled_contexts = default_enabled_context_keys(&self.contexts).into_iter().collect();
        } else {
            self.enabled_contexts
                .retain(|context_key| self.contexts.iter().any(|context| &context.key == context_key));
            if self.enabled_contexts.is_empty() {
                self.enabled_contexts =
                    default_enabled_context_keys(&self.contexts).into_iter().collect();
            }
        }

        let enabled_contexts: Vec<_> = self
            .contexts
            .iter()
            .filter(|context| self.enabled_contexts.contains(&context.key))
            .cloned()
            .collect();
        self.tabs = build_tabs(&enabled_contexts);
        self.available_contexts = self
            .contexts
            .iter()
            .filter(|context| !self.enabled_contexts.contains(&context.key))
            .map(|context| AvailableContextDescriptor {
                context_key: context.key.clone(),
                title: context.title.clone(),
                icon_key: context.icon_key.clone(),
                status: context.status,
            })
            .collect();
        self.active_page_by_context.clear();

        for tab in &self.tabs {
            let preserved_page = previous_active_page_by_context
                .get(&tab.context_key)
                .copied()
                .filter(|page_id| tab.pages.iter().any(|page| page.id == *page_id));

            if let Some(page_id) = preserved_page.or_else(|| tab.pages.first().map(|page| page.id)) {
                self.active_page_by_context
                    .insert(tab.context_key.clone(), page_id);
            }
        }

        self.active_context_key = previous_active_context
            .filter(|context_key| self.tabs.iter().any(|tab| &tab.context_key == context_key))
            .or_else(|| self.tabs.first().map(|tab| tab.context_key.clone()));
    }
}
