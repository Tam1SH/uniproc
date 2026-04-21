use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::Message;
use app_core::trace::in_named_scope;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PageId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct TabId(pub u32);

impl Default for PageId {
    fn default() -> Self {
        PageId(0)
    }
}

impl Default for TabId {
    fn default() -> Self {
        TabId(0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum PageStatus {
    #[default]
    Inactive,
    Loading,
    Ready,
    Error,
}

#[derive(Clone, Debug)]
pub struct PageStatusChanged {
    pub tab_id: TabId,
    pub page_id: PageId,
    pub status: PageStatus,
    pub error: Option<String>,
}

impl Message for PageStatusChanged {}

#[derive(Clone, Debug)]
pub struct TabStatusChanged {
    pub tab_id: TabId,
    pub status: PageStatus,
    pub error: Option<String>,
}

impl Message for TabStatusChanged {}

#[derive(Clone, Debug)]
pub struct FeatureState {
    pub status: PageStatus,
    pub error_msg: String,
}

pub struct PageStatusRegistry {
    page_states: RwLock<HashMap<(TabId, PageId), FeatureState>>,
    tab_states: RwLock<HashMap<TabId, FeatureState>>,
}

impl PageStatusRegistry {
    pub fn new() -> Self {
        Self {
            page_states: RwLock::new(HashMap::new()),
            tab_states: RwLock::new(HashMap::new()),
        }
    }

    pub fn update_page(
        &self,
        tab_id: TabId,
        page_id: PageId,
        status: PageStatus,
        error: Option<String>,
    ) -> bool {
        let mut map = self.page_states.write().unwrap();
        let entry = map.entry((tab_id, page_id)).or_insert(FeatureState {
            status: PageStatus::Loading,
            error_msg: String::new(),
        });

        let new_error = error.unwrap_or_default();

        if entry.status == status && entry.error_msg == new_error {
            return false;
        }

        entry.status = status;
        entry.error_msg = new_error;
        true
    }

    pub fn update_tab(&self, tab_id: TabId, status: PageStatus, error: Option<String>) -> bool {
        let mut map = self.tab_states.write().unwrap();
        let entry = map.entry(tab_id).or_insert(FeatureState {
            status: PageStatus::Loading,
            error_msg: String::new(),
        });

        let new_error = error.unwrap_or_default();

        if entry.status == status && entry.error_msg == new_error {
            return false;
        }

        entry.status = status;
        entry.error_msg = new_error;
        true
    }

    pub fn get_page_state(&self, tab_id: TabId, page_id: PageId) -> FeatureState {
        let map = self.page_states.read().unwrap();
        map.get(&(tab_id, page_id))
            .cloned()
            .unwrap_or(FeatureState {
                status: PageStatus::Loading,
                error_msg: String::new(),
            })
    }

    pub fn get_tab_state(&self, tab_id: TabId) -> FeatureState {
        let map = self.tab_states.read().unwrap();
        map.get(&tab_id).cloned().unwrap_or(FeatureState {
            status: PageStatus::Loading,
            error_msg: String::new(),
        })
    }

    pub fn report_page(&self, msg: PageStatusChanged) {
        in_named_scope(
            "context.page_status.update",
            Some("tab_id,page_id,status"),
            Some(format!(
                "{:?} | {:?} | {:?}",
                msg.tab_id, msg.page_id, msg.status
            )),
            || {
                if self.update_page(msg.tab_id, msg.page_id, msg.status, msg.error.clone()) {
                    EventBus::publish(msg);
                }
            },
        );
    }

    pub fn report_tab(&self, msg: TabStatusChanged) {
        in_named_scope(
            "context.page_status.update",
            Some("tab_id,status"),
            Some(format!("{:?} | {:?}", msg.tab_id, msg.status)),
            || {
                if self.update_tab(msg.tab_id, msg.status, msg.error.clone()) {
                    EventBus::publish(msg);
                }
            },
        );
    }
}
