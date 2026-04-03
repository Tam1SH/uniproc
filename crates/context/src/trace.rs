use anyhow::Context;
use app_core::trace::{register_scopes, TracePolicy};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{Event, Id, Level, Subscriber};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format;
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::layer::{Context as LayerContext, Layer};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

include!(concat!(env!("OUT_DIR"), "/trace_scopes.rs"));

#[derive(Clone, Default)]
struct TraceFields {
    message: Option<String>,
    scope: Option<String>,
    correlation_id: Option<String>,
    op_id: Option<u64>,
    scope_target: Option<String>,
}

#[derive(Clone)]
pub struct TraceDumpLayer {
    state: Arc<Mutex<DumpState>>,
}

impl TraceDumpLayer {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(DumpState {
                capacity: capacity.max(1),
                entries: HashMap::new(),
            })),
        }
    }
}

impl<S> Layer<S> for TraceDumpLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &Id,
        ctx: LayerContext<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = TraceFieldVisitor::default();
            attrs.record(&mut visitor);
            span.extensions_mut().insert(visitor.finish());
        }
    }

    fn on_record(&self, id: &Id, values: &tracing::span::Record<'_>, ctx: LayerContext<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = TraceFieldVisitor::default();
            values.record(&mut visitor);
            let update = visitor.finish();
            let mut exts = span.extensions_mut();
            if let Some(existing) = exts.get_mut::<TraceFields>() {
                merge_fields(existing, &update);
            } else {
                exts.insert(update);
            }
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: LayerContext<'_, S>) {
        let mut visitor = TraceFieldVisitor::default();
        event.record(&mut visitor);
        let mut fields = visitor.finish();

        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(span_fields) = span.extensions().get::<TraceFields>() {
                    merge_missing(&mut fields, span_fields);
                }
            }
        }

        let Some(key) = correlation_key(&fields) else {
            return;
        };

        let level = *event.metadata().level();
        let summary = render_summary(level, &fields);
        let mut state = self.state.lock().expect("trace dump lock poisoned");

        if level <= tracing::Level::INFO {
            let capacity = state.capacity;
            let entries = state.entries.entry(key).or_default();
            entries.push_back(summary);
            while entries.len() > capacity {
                entries.pop_front();
            }
            return;
        }

        if let Some(entries) = state.entries.remove(&key)
            && !entries.is_empty()
        {
            eprintln!("trace dump start key={key}");
            for entry in entries {
                eprintln!("  {entry}");
            }
            eprintln!("trace dump end key={key}");
        }
    }
}

pub fn install_defaults() {
    register_scopes(ALL_SCOPES);
}

pub fn init_subscriber<W>(settings_path: &Path, writer: W) -> anyhow::Result<()>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    install_defaults();

    let trace_policy = load_policy(settings_path);
    let dump_capacity = trace_policy.dump_capacity;
    app_core::trace::install_policy(trace_policy);

    tracing_subscriber::registry()
        .with(default_targets())
        .with(TraceDumpLayer::new(dump_capacity))
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(format().compact())
                .with_target(false)
                .with_file(false)
                .with_line_number(false)
                .with_writer(writer),
        )
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok(())
}

pub fn load_policy(path: &Path) -> TracePolicy {
    let mut policy = TracePolicy::default();

    if let Ok(raw) = std::fs::read_to_string(path)
        && let Ok(value) = serde_json::from_str::<Value>(&raw)
    {
        extend_strings(
            &mut policy.enabled_prefixes,
            value.pointer("/trace/enable_scopes"),
        );
        extend_strings(
            &mut policy.disabled_prefixes,
            value.pointer("/trace/disable_scopes"),
        );

        if let Some(capacity) = value
            .pointer("/trace/dump_capacity")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .filter(|value| *value > 0)
        {
            policy.dump_capacity = capacity;
        }
    }

    extend_env_json(
        &mut policy.enabled_prefixes,
        std::env::var("UNIPROC_TRACE_ENABLE_SCOPES").ok(),
    );
    extend_env_json(
        &mut policy.disabled_prefixes,
        std::env::var("UNIPROC_TRACE_DISABLE_SCOPES").ok(),
    );

    if let Some(capacity) = std::env::var("UNIPROC_TRACE_DUMP_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
    {
        policy.dump_capacity = capacity;
    }

    dedup(&mut policy.enabled_prefixes);
    dedup(&mut policy.disabled_prefixes);
    policy
}

#[derive(Default)]
struct TraceFieldVisitor {
    fields: TraceFields,
}

impl TraceFieldVisitor {
    fn finish(self) -> TraceFields {
        self.fields
    }
}

impl tracing::field::Visit for TraceFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.record_value(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "op_id" {
            self.fields.op_id = Some(value);
        } else {
            self.record_value(field.name(), value.to_string());
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.record_value(field.name(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.record_value(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.record_value(field.name(), format!("{value:?}"));
    }
}

impl TraceFieldVisitor {
    fn record_value(&mut self, name: &str, value: String) {
        match name {
            "message" => self.fields.message = Some(trim_debug_string(value)),
            "scope" => self.fields.scope = Some(trim_debug_string(value)),
            "correlation_id" => self.fields.correlation_id = Some(trim_debug_string(value)),
            "target" => self.fields.scope_target = Some(trim_debug_string(value)),
            "op_id" => {
                self.fields.op_id = trim_debug_string(value).parse::<u64>().ok();
            }
            _ => {}
        }
    }
}

struct DumpState {
    capacity: usize,
    entries: HashMap<String, VecDeque<String>>,
}

fn extend_strings(dst: &mut Vec<String>, value: Option<&Value>) {
    let Some(Value::Array(items)) = value else {
        return;
    };

    for item in items {
        if let Some(scope) = item.as_str().map(str::trim).filter(|scope| !scope.is_empty()) {
            dst.push(scope.to_string());
        }
    }
}

fn extend_env_json(dst: &mut Vec<String>, raw: Option<String>) {
    let Some(raw) = raw else {
        return;
    };

    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Array(items)) => {
            for item in items {
                if let Some(scope) = item.as_str().map(str::trim).filter(|scope| !scope.is_empty())
                {
                    dst.push(scope.to_string());
                }
            }
        }
        Ok(Value::String(scope)) => {
            let scope = scope.trim();
            if !scope.is_empty() {
                dst.push(scope.to_string());
            }
        }
        _ => {
            let scope = raw.trim();
            if !scope.is_empty() {
                dst.push(scope.to_string());
            }
        }
    }
}

fn dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn default_targets() -> Targets {
    Targets::new()
        .with_default(Level::DEBUG)
        .with_target("ogurpchik", Level::WARN)
        .with_target("app_core::settings::store", Level::WARN)
        .with_target("context::caches::icons::windows", Level::ERROR)
}

fn merge_fields(dst: &mut TraceFields, src: &TraceFields) {
    if src.message.is_some() {
        dst.message = src.message.clone();
    }
    if src.scope.is_some() {
        dst.scope = src.scope.clone();
    }
    if src.correlation_id.is_some() {
        dst.correlation_id = src.correlation_id.clone();
    }
    if src.op_id.is_some() {
        dst.op_id = src.op_id;
    }
    if src.scope_target.is_some() {
        dst.scope_target = src.scope_target.clone();
    }
}

fn merge_missing(dst: &mut TraceFields, src: &TraceFields) {
    if dst.scope.is_none() {
        dst.scope = src.scope.clone();
    }
    if dst.correlation_id.is_none() {
        dst.correlation_id = src.correlation_id.clone();
    }
    if dst.op_id.is_none() {
        dst.op_id = src.op_id;
    }
    if dst.scope_target.is_none() {
        dst.scope_target = src.scope_target.clone();
    }
}

fn correlation_key(fields: &TraceFields) -> Option<String> {
    fields
        .correlation_id
        .as_ref()
        .filter(|value| !value.is_empty())
        .cloned()
        .or_else(|| fields.op_id.map(|value| format!("op:{value}")))
}

fn render_summary(level: tracing::Level, fields: &TraceFields) -> String {
    let scope = fields.scope.as_deref().unwrap_or("-");
    let message = fields.message.as_deref().unwrap_or("-");
    let corr = fields.correlation_id.as_deref().unwrap_or("-");
    let op_id = fields
        .op_id
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let target = fields.scope_target.as_deref().unwrap_or("-");

    format!(
        "{level:<5} scope={scope} op={op_id} corr={corr} target={target} msg={message}"
    )
}

fn trim_debug_string(value: String) -> String {
    value.trim_matches('"').to_string()
}
