use anyhow::Context;
use app_core::trace::{register_scopes, TracePolicy};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use tracing::{Event, Id, Level, Subscriber};
use tracing_subscriber::field::Visit;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};
use tracing_subscriber::fmt::writer::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::layer::{Context as LayerContext, Layer};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;

include!(concat!(env!("OUT_DIR"), "/trace_scopes.rs"));

static LAST_CONSOLE_FINGERPRINT: LazyLock<Mutex<Option<String>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Default)]
struct TraceFields {
    message: Option<String>,
    scope: Option<String>,
    correlation_id: Option<String>,
    op_id: Option<u64>,
    target_fields: Option<String>,
    scope_target: Option<String>,
}

#[derive(Default)]
struct PlainFields {
    values: Vec<(String, String)>,
}

struct PlainFieldVisitor<'a> {
    values: &'a mut Vec<(String, String)>,
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
        let scope_chain = scope_chain_for_event(&ctx, event);

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
        let plain_fields = collect_plain_fields(event);
        let fingerprint = dump_fingerprint(&fields, scope_chain.as_deref(), &plain_fields);
        let mut state = self.state.lock().expect("trace dump lock poisoned");

        if matches!(
            level,
            tracing::Level::TRACE | tracing::Level::DEBUG | tracing::Level::INFO
        ) {
            let capacity = state.capacity;
            let entries = state.entries.entry(key).or_default();
            push_dump_entry(
                entries,
                DumpEntry {
                    fingerprint,
                    repeats: 1,
                },
            );
            while entries.len() > capacity {
                entries.pop_front();
            }
            return;
        }

        if let Some(entries) = state.entries.remove(&key)
            && !entries.is_empty()
        {
            let repeats = entries.iter().map(|entry| entry.repeats).sum::<usize>();
            eprintln!(
                "trace dump available key={} entries={} unique={}",
                short_correlation_id(&key),
                repeats,
                entries.len()
            );
        }
    }
}

pub fn install_defaults() {
    register_scopes(ALL_SCOPES);
}

pub fn builtin_policy() -> TracePolicy {
    normalize_policy(TracePolicy {
        enabled_prefixes: BUILTIN_ENABLE_SCOPES
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        disabled_prefixes: Vec::new(),
        disabled_message_prefixes: BUILTIN_DISABLE_MESSAGES
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        disabled_target_prefixes: BUILTIN_DISABLE_TARGETS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        dump_capacity: TracePolicy::default().dump_capacity,
    })
}

pub fn init_subscriber<W>(settings_path: &Path, writer: W) -> anyhow::Result<()>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    install_defaults();

    let trace_policy = load_policy(settings_path);
    let dump_capacity = trace_policy.dump_capacity;
    app_core::trace::install_policy(trace_policy);

    init_internal(writer, dump_capacity, None)
}

pub fn init_test_subscriber<W>(
    writer: W,
    test_storage: Arc<Mutex<Vec<String>>>,
) -> anyhow::Result<()>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    install_defaults();

    let dump_capacity = 64;

    init_internal(writer, dump_capacity, Some(test_storage))
}

fn init_internal<W>(
    writer: W,
    dump_capacity: usize,
    test_storage: Option<Arc<Mutex<Vec<String>>>>,
) -> anyhow::Result<()>
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let test_layer = test_storage.map(|storage| TestCaptureLayer(storage));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .event_format(CompactTraceFormatter)
        .with_writer(writer);

    tracing_subscriber::registry()
        .with(default_targets())
        .with(TraceDumpLayer::new(dump_capacity))
        .with(fmt_layer)
        .with(test_layer)
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok(())
}

pub fn load_policy(path: &Path) -> TracePolicy {
    let mut policy = builtin_policy();

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
        extend_strings(
            &mut policy.disabled_message_prefixes,
            value.pointer("/trace/disable_messages"),
        );
        extend_strings(
            &mut policy.disabled_target_prefixes,
            value.pointer("/trace/disable_targets"),
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

    normalize_policy(policy)
}

pub fn normalize_policy(mut policy: TracePolicy) -> TracePolicy {
    dedup(&mut policy.enabled_prefixes);
    dedup(&mut policy.disabled_prefixes);
    dedup(&mut policy.disabled_message_prefixes);
    dedup(&mut policy.disabled_target_prefixes);
    if policy.dump_capacity == 0 {
        policy.dump_capacity = TracePolicy::default().dump_capacity;
    }
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

impl Visit for TraceFieldVisitor {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.record_value(field.name(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "op_id" {
            self.fields.op_id = Some(value);
        } else {
            self.record_value(field.name(), value.to_string());
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.record_value(field.name(), value.to_string());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
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
            "target_fields" => self.fields.target_fields = Some(trim_debug_string(value)),
            "target" => self.fields.scope_target = Some(trim_debug_string(value)),
            "op_id" => {
                self.fields.op_id = trim_debug_string(value).parse::<u64>().ok();
            }
            _ => {}
        }
    }
}

impl Visit for PlainFieldVisitor<'_> {
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.values
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.values
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.values
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.values.push((
            field.name().to_string(),
            trim_debug_string(value.to_string()),
        ));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.values.push((
            field.name().to_string(),
            trim_debug_string(format!("{value:?}")),
        ));
    }
}

struct DumpState {
    capacity: usize,
    entries: HashMap<String, VecDeque<DumpEntry>>,
}

#[derive(Clone)]
struct DumpEntry {
    fingerprint: String,
    repeats: usize,
}

fn extend_strings(dst: &mut Vec<String>, value: Option<&Value>) {
    let Some(Value::Array(items)) = value else {
        return;
    };

    for item in items {
        if let Some(scope) = item
            .as_str()
            .map(str::trim)
            .filter(|scope| !scope.is_empty())
        {
            dst.push(scope.to_string());
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
    if src.target_fields.is_some() {
        dst.target_fields = src.target_fields.clone();
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
    if dst.target_fields.is_none() {
        dst.target_fields = src.target_fields.clone();
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

fn dump_fingerprint(
    fields: &TraceFields,
    scope_chain: Option<&str>,
    plain_fields: &HashMap<String, String>,
) -> String {
    let scope = scope_chain.unwrap_or_else(|| fields.scope.as_deref().unwrap_or("-"));
    let message = fields.message.as_deref().unwrap_or("-");
    let target = fields.scope_target.as_deref().unwrap_or("-");
    let target_fields = fields.target_fields.as_deref().unwrap_or("-");
    let extra = ordered_plain_fields(plain_fields)
        .into_iter()
        .map(|(name, value)| format!("{name}={}", clean_display_value(value)))
        .collect::<Vec<_>>()
        .join("|");

    format!("{scope}|{target}|{target_fields}|{message}|{extra}")
}

fn trim_debug_string(value: String) -> String {
    value.trim_matches('"').to_string()
}

struct CompactTraceFormatter;

impl<S, N> FormatEvent<S, N> for CompactTraceFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let mut fields = TraceFieldVisitor::default();
        event.record(&mut fields);
        let mut trace_fields = fields.finish();

        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                if let Some(span_fields) = span.extensions().get::<TraceFields>() {
                    merge_missing(&mut trace_fields, span_fields);
                }
            }
        }

        let plain_fields = collect_plain_fields(event);
        let scope_display = scope_chain_for_context(ctx)
            .or_else(|| {
                trace_fields
                    .scope
                    .as_deref()
                    .filter(|scope| !scope.is_empty())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "-".to_string());
        let console_fingerprint = console_fingerprint(
            meta.level(),
            &scope_display,
            trace_fields.scope_target.as_deref(),
            plain_fields.get("actor").map(String::as_str),
            plain_fields.get("event").map(String::as_str),
            trace_fields.message.as_deref(),
            &plain_fields,
        );

        if should_skip_console_fingerprint(console_fingerprint) {
            return Ok(());
        }

        write!(writer, "\x1b[90m")?;
        SystemTime.format_time(&mut writer)?;
        write!(writer, "\x1b[0m ")?;
        write_colored(
            &mut writer,
            level_color(meta.level()),
            format_args!("{:>5}", meta.level()),
        )?;

        if let Some(actor) = plain_fields.get("actor").filter(|value| !value.is_empty()) {
            write!(writer, " ")?;
            write_colored(
                &mut writer,
                Color::Cyan,
                format_args!("actor={}", clean_display_value(actor)),
            )?;
        }

        if let Some(event_name) = plain_fields.get("event").filter(|value| !value.is_empty()) {
            write!(writer, " ")?;
            write_colored(
                &mut writer,
                Color::Purple,
                format_args!("event={}", clean_display_value(event_name)),
            )?;
        }

        if scope_display != "-" {
            write!(writer, " ")?;
            write_colored(&mut writer, Color::Blue, format_args!("[{scope_display}]"))?;
        }

        if let Some(target) = trace_fields
            .scope_target
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            write!(writer, " ")?;
            write_colored(
                &mut writer,
                Color::Purple,
                format_args!("[{}]", clean_display_value(target)),
            )?;
        }

        let mut service_meta = Vec::new();
        if let Some(op_id) = trace_fields.op_id {
            service_meta.push(format!("op={op_id}"));
        }
        if let Some(correlation_id) = trace_fields
            .correlation_id
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            service_meta.push(format!("corr={}", short_correlation_id(correlation_id)));
        }
        if let Some(target_fields) = trace_fields
            .target_fields
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            service_meta.push(format!("fields={target_fields}"));
        }
        if !service_meta.is_empty() {
            write!(writer, " ")?;
            write_colored(
                &mut writer,
                Color::Gray,
                format_args!("({})", service_meta.join(" ")),
            )?;
        }

        for (name, value) in ordered_plain_fields(&plain_fields) {
            if value.is_empty() {
                continue;
            }
            write!(writer, " ")?;
            write_colored(
                &mut writer,
                field_color(name),
                format_args!("{name}={value}"),
            )?;
        }

        if let Some(message) = trace_fields
            .message
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            write!(writer, " {message}")?;
        }

        writeln!(writer)
    }
}

fn scope_chain_for_event<S>(ctx: &LayerContext<'_, S>, event: &Event<'_>) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    ctx.event_scope(event)
        .map(|scope| scope_chain_from_spans(scope.from_root()))
}

fn scope_chain_for_context<S, N>(
    ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    ctx.event_scope()
        .map(|scope| scope_chain_from_spans(scope.from_root()))
}

fn scope_chain_from_spans<'a, S, I>(spans: I) -> String
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    I: IntoIterator<Item = tracing_subscriber::registry::SpanRef<'a, S>>,
{
    let mut parts = Vec::new();

    for span in spans {
        if let Some(fields) = span.extensions().get::<TraceFields>()
            && let Some(scope) = fields.scope.as_deref().filter(|scope| !scope.is_empty())
            && parts.last().is_none_or(|prev| prev != scope)
        {
            parts.push(scope.to_string());
        }
    }

    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" -> ")
    }
}

fn dedup_plain_fields(values: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = HashMap::<String, String>::new();
    let mut ordered = Vec::new();

    for (name, value) in values {
        if matches!(
            name.as_str(),
            "message" | "scope" | "correlation_id" | "op_id" | "target_fields" | "target"
        ) {
            continue;
        }

        if seen.get(&name).is_some_and(|existing| existing == &value) {
            continue;
        }

        seen.insert(name.clone(), value.clone());
        ordered.push((name, value));
    }

    ordered
}

fn collect_plain_fields(event: &Event<'_>) -> HashMap<String, String> {
    let mut plain = PlainFields::default();
    let mut visitor = PlainFieldVisitor {
        values: &mut plain.values,
    };
    event.record(&mut visitor);

    dedup_plain_fields(plain.values).into_iter().collect()
}

fn ordered_plain_fields<'a>(fields: &'a HashMap<String, String>) -> Vec<(&'a str, &'a str)> {
    let mut ordered = Vec::new();

    for key in ["result", "status", "pids", "cols", "gap_s", "timeout_s"] {
        if let Some(value) = fields.get(key) {
            ordered.push((key, value.as_str()));
        }
    }

    let mut rest = fields
        .iter()
        .filter(|(key, _)| {
            !matches!(
                key.as_str(),
                "actor" | "event" | "result" | "status" | "pids" | "cols" | "gap_s" | "timeout_s"
            )
        })
        .collect::<Vec<_>>();
    rest.sort_by(|a, b| a.0.cmp(b.0));

    for (key, value) in rest {
        ordered.push((key.as_str(), value.as_str()));
    }

    ordered
}

fn console_fingerprint(
    level: &Level,
    scope: &str,
    target: Option<&str>,
    actor: Option<&str>,
    event: Option<&str>,
    message: Option<&str>,
    plain_fields: &HashMap<String, String>,
) -> String {
    let mut parts = vec![
        level.as_str().to_string(),
        scope.to_string(),
        clean_display_value(target.unwrap_or("-")).to_string(),
        clean_display_value(actor.unwrap_or("-")).to_string(),
        clean_display_value(event.unwrap_or("-")).to_string(),
        message.unwrap_or("-").to_string(),
    ];

    for (name, value) in ordered_plain_fields(plain_fields) {
        parts.push(format!("{name}={}", clean_display_value(value)));
    }

    parts.join("|")
}

fn should_skip_console_fingerprint(fingerprint: String) -> bool {
    let mut last = LAST_CONSOLE_FINGERPRINT
        .lock()
        .expect("console fingerprint lock poisoned");

    if last.as_deref() == Some(fingerprint.as_str()) {
        true
    } else {
        *last = Some(fingerprint);
        false
    }
}

fn short_correlation_id(value: &str) -> &str {
    value.split('-').next().unwrap_or(value)
}

fn clean_display_value(value: &str) -> &str {
    value.trim_end_matches('>')
}

#[derive(Clone, Copy)]
enum Color {
    Blue,
    Cyan,
    Gray,
    Green,
    Purple,
    Red,
    White,
    Yellow,
}

fn level_color(level: &Level) -> Color {
    match *level {
        Level::ERROR => Color::Red,
        Level::WARN => Color::Yellow,
        Level::INFO => Color::Green,
        Level::DEBUG => Color::White,
        Level::TRACE => Color::Gray,
    }
}

fn field_color(name: &str) -> Color {
    match name {
        "adapter" | "method" => Color::Purple,
        "result" | "status" => Color::Green,
        "pids" | "cols" | "gap_s" | "timeout_s" => Color::Yellow,
        _ => Color::Gray,
    }
}

fn color_code(color: Color) -> &'static str {
    match color {
        Color::Blue => "\x1b[94m",
        Color::Cyan => "\x1b[96m",
        Color::Gray => "\x1b[90m",
        Color::Green => "\x1b[92m",
        Color::Purple => "\x1b[95m",
        Color::Red => "\x1b[91m",
        Color::White => "\x1b[97m",
        Color::Yellow => "\x1b[93m",
    }
}

fn write_colored(writer: &mut Writer<'_>, color: Color, args: fmt::Arguments<'_>) -> fmt::Result {
    write!(writer, "{}{}\x1b[0m", color_code(color), args)
}

fn push_dump_entry(entries: &mut VecDeque<DumpEntry>, incoming: DumpEntry) {
    if let Some(last) = entries.back_mut()
        && last.fingerprint == incoming.fingerprint
    {
        last.repeats += incoming.repeats;
        return;
    }

    entries.push_back(incoming);
}

pub struct TestCaptureLayer(Arc<Mutex<Vec<String>>>);

impl<S> Layer<S> for TestCaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
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

        let scope_chain = scope_chain_for_event(&ctx, event);
        let plain_fields = collect_plain_fields(event);

        let fingerprint = dump_fingerprint(&fields, scope_chain.as_deref(), &plain_fields);

        if let Ok(mut logs) = self.0.lock() {
            logs.push(fingerprint);
        }
    }
}
