use app_core::actor::traits::Message;
use context::page_status::{PageId, PageStatus, TabId};
use std::borrow::Cow;

pub mod page_ids {
    use super::PageId;
    pub const DUMMY: PageId = PageId(0);
    pub const PROCESSES: PageId = PageId(1);
    pub const PERFORMANCE: PageId = PageId(2);
    pub const DISK: PageId = PageId(3);
    pub const STATISTICS: PageId = PageId(4);
    pub const STARTUP_APPS: PageId = PageId(5);
    pub const USERS: PageId = PageId(6);
    pub const SERVICES: PageId = PageId(7);
}

pub mod tab_ids {
    use super::TabId;
    pub const MAIN: TabId = TabId(0);
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TabContextKey(pub Cow<'static, str>);

impl TabContextKey {
    pub const HOST: TabContextKey = TabContextKey(Cow::Borrowed("host/windows"));
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TabContextKind {
    #[default]
    Host,
    Wsl,
    Docker,
    Custom(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CapabilityStatus {
    #[default]
    Available,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum CapabilityValue {
    #[default]
    None,
    Flag(bool),
    Number(i64),
    Text(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapabilityProperty {
    pub key: String,
    pub value: CapabilityValue,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub title: String,
    pub status: CapabilityStatus,
    pub tags: Vec<String>,
    pub properties: Vec<CapabilityProperty>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabContextSnapshot {
    pub key: TabContextKey,
    pub kind: TabContextKind,
    pub title: String,
    pub icon_key: String,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub status: PageStatus,
    pub error_msg: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageDescriptor {
    pub id: PageId,
    pub text: String,
    pub icon_key: String,
    pub status: PageStatus,
    pub error_msg: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabDescriptor {
    pub id: TabId,
    pub context_key: TabContextKey,
    pub title: String,
    pub icon_key: String,
    pub pages: Vec<PageDescriptor>,
    pub status: PageStatus,
    pub error_msg: String,
    pub is_closable: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AvailableContextDescriptor {
    pub context_key: TabContextKey,
    pub title: String,
    pub icon_key: String,
    pub status: PageStatus,
}

#[derive(Clone, Debug)]
pub struct PageActivated {
    pub tab_id: TabId,
    pub page_id: PageId,
}

impl Message for PageActivated {}

#[derive(Clone, Debug)]
pub struct TabActivated {
    pub tab_id: TabId,
}

impl Message for TabActivated {}

#[derive(Clone, Debug)]
pub struct NavigationContextsChanged {
    pub contexts: Vec<TabContextSnapshot>,
}

impl Message for NavigationContextsChanged {}
