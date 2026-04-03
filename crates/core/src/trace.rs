use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{Level, Span};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    Core,
    Ui,
    Context,
}

#[derive(Clone, Copy, Debug)]
pub struct ScopeSpec {
    pub name: &'static str,
    pub kind: ScopeKind,
    pub enabled_by_default: bool,
}

impl ScopeSpec {
    pub const fn new(name: &'static str, kind: ScopeKind) -> Self {
        Self {
            name,
            kind,
            enabled_by_default: true,
        }
    }

    pub const fn disabled(name: &'static str, kind: ScopeKind) -> Self {
        Self {
            name,
            kind,
            enabled_by_default: false,
        }
    }
}

#[derive(Clone)]
pub struct DispatchMeta {
    pub op_id: u64,
    pub correlation_id: Option<String>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct TracePolicy {
    pub enabled_prefixes: Vec<String>,
    pub disabled_prefixes: Vec<String>,
    pub dump_capacity: usize,
}

impl Default for TracePolicy {
    fn default() -> Self {
        Self {
            enabled_prefixes: Vec::new(),
            disabled_prefixes: Vec::new(),
            dump_capacity: 64,
        }
    }
}

static NEXT_OP_ID: AtomicU64 = AtomicU64::new(1);
static SCOPE_REGISTRY: Lazy<RwLock<HashMap<&'static str, ScopeSpec>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static TRACE_POLICY: Lazy<RwLock<TracePolicy>> = Lazy::new(|| RwLock::new(TracePolicy::default()));

thread_local! {
    static CURRENT_META: RefCell<Option<DispatchMeta>> = const { RefCell::new(None) };
}

impl DispatchMeta {
    pub fn capture_or_root(source: &'static str) -> Self {
        current_meta().unwrap_or_else(|| Self::root(source, None, None))
    }

    pub fn root(
        scope: &'static str,
        target_fields: Option<&'static str>,
        target: Option<String>,
    ) -> Self {
        let op_id = next_op_id();
        let correlation_id = Uuid::new_v4().to_string();
        let span = span_for_scope(
            scope,
            op_id,
            Some(correlation_id.as_str()),
            target_fields,
            target.as_deref(),
        );

        Self {
            op_id,
            correlation_id: Some(correlation_id),
            span,
        }
    }

    pub fn child(
        &self,
        scope: &'static str,
        target_fields: Option<&'static str>,
        target: Option<String>,
    ) -> Self {
        let correlation_id = self.correlation_id.clone();
        let span = span_for_child(
            &self.span,
            scope,
            self.op_id,
            correlation_id.as_deref(),
            target_fields,
            target.as_deref(),
        );

        Self {
            op_id: self.op_id,
            correlation_id,
            span,
        }
    }
}

pub fn register_scopes(scopes: &'static [ScopeSpec]) {
    let mut registry = SCOPE_REGISTRY.write();
    for scope in scopes {
        registry.insert(scope.name, *scope);
    }
}

pub fn install_policy(policy: TracePolicy) {
    *TRACE_POLICY.write() = policy;
}

pub fn current_policy() -> TracePolicy {
    TRACE_POLICY.read().clone()
}

pub fn is_scope_enabled(scope: &str) -> bool {
    let policy = TRACE_POLICY.read().clone();
    if let Some(enabled) = resolve_prefix_override(scope, &policy.enabled_prefixes, true) {
        return enabled;
    }
    if let Some(enabled) = resolve_prefix_override(scope, &policy.disabled_prefixes, false) {
        return enabled;
    }

    SCOPE_REGISTRY
        .read()
        .get(scope)
        .map(|spec| spec.enabled_by_default)
        .unwrap_or(true)
}

pub fn current_meta() -> Option<DispatchMeta> {
    CURRENT_META.with(|slot| slot.borrow().clone())
}

pub fn install_current_meta(meta: DispatchMeta) -> MetaGuard {
    let prev = CURRENT_META.with(|slot| slot.replace(Some(meta)));
    MetaGuard { prev }
}

pub fn current_correlation_id() -> Option<String> {
    current_meta().and_then(|meta| meta.correlation_id)
}

pub fn current_or_new_correlation_uuid() -> Uuid {
    current_correlation_id()
        .and_then(|id| Uuid::parse_str(&id).ok())
        .unwrap_or_else(Uuid::new_v4)
}

pub fn in_named_scope<R>(
    scope: &'static str,
    target_fields: Option<&'static str>,
    target: Option<String>,
    f: impl FnOnce() -> R,
) -> R {
    let meta = current_meta()
        .map(|meta| meta.child(scope, target_fields, target.clone()))
        .unwrap_or_else(|| DispatchMeta::root(scope, target_fields, target));
    let _meta_guard = install_current_meta(meta.clone());
    let _enter = meta.span.enter();
    f()
}

pub fn in_ui_action_scope<R>(
    scope: &'static str,
    target_fields: Option<&'static str>,
    target: Option<String>,
    f: impl FnOnce() -> R,
) -> R {
    let meta = DispatchMeta::root(scope, target_fields, target);
    let _meta_guard = install_current_meta(meta.clone());
    let _enter = meta.span.enter();
    f()
}

pub fn format_ui_target_1<T>(value: &T) -> Option<String>
where
    T: Debug + ?Sized,
{
    Some(format!("{value:?}"))
}

pub fn format_ui_target_2<A, B>(left: &A, right: &B) -> Option<String>
where
    A: Debug + ?Sized,
    B: Debug + ?Sized,
{
    Some(format!("{left:?} | {right:?}"))
}

pub struct MetaGuard {
    prev: Option<DispatchMeta>,
}

impl Drop for MetaGuard {
    fn drop(&mut self) {
        CURRENT_META.with(|slot| {
            slot.replace(self.prev.take());
        });
    }
}

fn next_op_id() -> u64 {
    NEXT_OP_ID.fetch_add(1, Ordering::Relaxed)
}

fn resolve_prefix_override(scope: &str, prefixes: &[String], value: bool) -> Option<bool> {
    prefixes
        .iter()
        .filter(|prefix| matches_scope_prefix(scope, prefix))
        .map(|prefix| prefix.len())
        .max()
        .map(|_| value)
}

fn matches_scope_prefix(scope: &str, prefix: &str) -> bool {
    scope == prefix
        || scope
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn span_for_scope(
    scope: &'static str,
    op_id: u64,
    correlation_id: Option<&str>,
    target_fields: Option<&'static str>,
    target: Option<&str>,
) -> Span {
    if !is_scope_enabled(scope) {
        return Span::none();
    }

    tracing::span!(
        Level::INFO,
        "scope",
        scope,
        op_id,
        correlation_id = correlation_id.unwrap_or(""),
        target_fields = target_fields.unwrap_or(""),
        target = target.unwrap_or(""),
    )
}

fn span_for_child(
    parent: &Span,
    scope: &'static str,
    op_id: u64,
    correlation_id: Option<&str>,
    target_fields: Option<&'static str>,
    target: Option<&str>,
) -> Span {
    if !is_scope_enabled(scope) {
        return parent.clone();
    }

    tracing::span!(
        parent: parent,
        Level::DEBUG,
        "scope",
        scope,
        op_id,
        correlation_id = correlation_id.unwrap_or(""),
        target_fields = target_fields.unwrap_or(""),
        target = target.unwrap_or(""),
    )
}
