use context::page_status::PageStatus;
use std::borrow::Cow;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TabContextKey(pub Cow<'static, str>);

impl TabContextKey {
    pub const HOST: TabContextKey = TabContextKey(Cow::Borrowed("host"));
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
pub struct TabPageDescriptor {
    pub path: String,
    pub route_segment: String,
    pub text: String,
    pub icon_key: String,
    pub status: PageStatus,
    pub error_msg: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabDescriptor {
    pub context_key: TabContextKey,
    pub title: String,
    pub icon_key: String,
    pub pages: Vec<TabPageDescriptor>,
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
