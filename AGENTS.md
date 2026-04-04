# AGENTS.md

Architecture guide for AI agents. Read this before touching any code.

---

## TL;DR

- Business logic lives in **actors** inside `domain`
- Features communicate only via **`EventBus`** — never call each other directly
- UI knows nothing about `domain` — only about `contract` (Port + Bindings traits)
- `install()` is the only bootstrap entry point for a feature
- Before build/check/run commands, inspect `.cargo/*.toml` for project aliases and toolchain config
- Prefer project cargo aliases for verification; default check command is `cargo cdev`
- When in doubt, copy `processes` or `services` as a reference

---

## Never edit manually

- `crates/context/src/icons.rs` — codegen'd icon registry from `slint-adapter/ui/assets`
- `crates/context/src/l10n.rs` — codegen'd from `.toml`
- `crates/context/src/trace.rs` scope catalog section — codegen'd from `crates/context/trace-scopes.toml`
- `slint-adapter/ui/shared/localization.slint` — codegen'd from `.toml`
- `slint-adapter/ui/shared/icons.slint` — codegen'd from `download.txt`

---

## What is this project

A task manager replacement. Rust, UI built with Slint.

Longer-term, treat it as a hub for system observability tools rather than only a task manager clone.

---

## Architecture vocabulary

Use these labels as shorthand for the current design. They are descriptive, not dogma.

- **Ports & Adapters / Hexagonal** — `domain` does not know Slint; UI integration lives behind `contract` traits and
  `slint-adapter`
- **Actor-based application layer** — features organize behavior around actors, messages and local actor state
- **Event-driven feature communication** — feature-to-feature interaction goes through `EventBus`, never direct calls
- **CQRS-style UI flow** — UI sends intents/commands via `Bindings`; domain pushes read/view state back via `Port`
- **Context as environment/infrastructure** — `context` owns settings, caches, page state, tracing policy and other
  service/runtime concerns, not business logic
- **UI-origin correlation** — user-initiated tracing/correlation starts in UI adapter callbacks; `app_core` only
  propagates it through runtime hops

---

## Crate structure

| Crate           | Role                                         | Depends on                               |
|-----------------|----------------------------------------------|------------------------------------------|
| `desktop`       | entry point, feature aggregation             | everything                               |
| `slint-adapter` | contract implementations for Slint           | `contract`, `core`                       |
| `contract`      | Port + Bindings traits, DTOs                 | `core`                                   |
| `domain/*`      | features, may be separate crates             | `core`, `context`, `widgets`, `contract` |
| `context`       | environment: settings, caches, page state    | `core`                                   |
| `widgets`       | shared UI code (tables)                      | `core`                                   |
| `core`          | actor system, event bus, Signal, SharedState | —                                        |

`context` and `widgets` have no knowledge of `slint-adapter` or `contract`. `slint-adapter` has no knowledge of
`domain`.

---

## Rules

- Business logic lives only in actors inside domain
- Features do not call each other directly — event bus only
- UI has no knowledge of domain, only of contract
- New feature = use an existing feature as the reference implementation
- Heavy feature = separate crate
- Do not add `contract` or `slint-adapter` dependencies to `context` or `widgets`
- Do not communicate with agents bypassing `AgentsFeature`
- `SharedState` is for bootstrap only, not business logic
- Do not invent new tracing conventions ad hoc — use the scope/correlation model described below
- Platform-dependent code must live in a dedicated subfolder/module, with one file per platform in the same place
- Prefer `windows.rs`, `linux.rs`, `macos.rs` (and similar) side-by-side over scattering `#[cfg(...)]` branches across unrelated files
- Keep the cross-platform entry point thin: `mod.rs` should select the platform module and expose the shared API

---

## Common tasks

**Build / check / run**
Start by reading `.cargo/config.toml` and any companion `.cargo/*.toml` files to pick the repo-supported command/flags.
Do not guess the command if the alias already exists.

- Default verification: `cargo cdev`
- Default dev build: `cargo bdev`
- Default dev run: `cargo rdev`
- Desktop build: `cargo bdesk`
- Desktop run: `cargo rdesk`

**Adding a feature**
Create a folder in `domain/src/features/` (or a new crate for heavy features), implement `Feature<TWindow>`, register in
`desktop/src/main.rs`. Reference: `domain/src/features/services/`.

**Adding a setting**
Add a field to `settings.rs` with `#[setting(default = ...)]`, rebuild. Use the generated getter in the actor.
Reference: `domain/src/features/processes/settings.rs`.

**Adding an icon**
Add a line to `slint-adapter/ui/assets/download.txt` in the format `name:url`, rebuild. Access via
`context::icons::Icons::get("name")` in Rust or the codegen'd Slint binding.

**Adding a locale string**
Edit `context/locales/*.toml`, rebuild. Do not touch any generated files.

**Adding a trace scope**
Edit `crates/context/trace-scopes.toml`, rebuild. Do not hand-edit the generated scope catalog in
`crates/context/src/trace.rs`.

**Adding a UI feature**
Create `slint-adapter/ui/features/my-feature/` with `index.slint` + `globals.slint`. Re-export the global from
`globals-export.slint`.

**Publishing a bus message**

```rust
EventBus::publish(MyMessage { ... }); // can be called from any thread
```

**Subscribing to a bus message**
Requires a `UiThreadGuard` (available in `install`) and an `Addr` to the actor:

```rust
let id = EventBus::subscribe( & guard, addr.clone()); // returns SubscriptionId
// store id — dropping it does NOT unsubscribe, call EventBus::unsubscribe(&guard, id) explicitly
```

The actor must implement `Handler<MyMessage, TWindow>`. Messages are always delivered on the UI thread via
`slint::invoke_from_event_loop`.

---

## core

Contains:

- **`Reactor`** — actor system, manages actor lifecycles
- **`EventBus`** — event bus, the only way features communicate with each other
- **`Signal<T>`** — reactive primitive, subscribe to value changes
- **`SharedState`** — typed dependency store for injecting shared state between features. **Internal**: bootstrap only,
  not for business logic
- **`App<TWindow>`** — container into which features are installed
- **`Feature<TWindow>`** trait — the contract every feature implements
- **`UiThreadGuard`** — token for UI-thread operations
### Feature trait

```rust
pub trait Feature<TWindow: Window>: Sized {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()>;
}
```

`install` is the single entry point for a feature. All bootstrap happens here.

### App and assembly

```rust
App::new(ui)
.feature(SettingsFeature::default ()) ?
.feature(AgentsFeature) ?
.feature(with_adapter!(NavigationFeature => NavigationUiAdapter)) ?
// ...
.run()
```

The `with_adapter!` macro is sugar for features that need a UI adapter:

```rust
macro_rules! with_adapter {
    ($feature:ident => $adapter:ident) => {
        $feature::new(|ui: &AppWindow| $adapter::new(ui.as_weak()))
    };
}
```

---

## context

The environment features operate in. Not UI, not business logic — environment.

Contains:

- **`SettingsStore`** — persistent JSON settings store
- **`ReactiveSetting<T>`** — a setting backed by `Signal<T>`: reads from the store, reacts to changes, writes back
- **`PageStatusRegistry`** — registry of page and tab states. Publishes `PageStatusChanged` / `TabStatusChanged` to the
  bus on change (with deduplication)
- **String caches** — buffers between UI and domain for string data
- **Icon cache** — extracts icons from processes
- **Locales** — `context/locales/*.toml`. To add new strings, edit only the `.toml` files. UI bindings and the Rust-side
  global are codegen'd automatically. The Rust side is currently unused.
- **Trace catalog + policy** — owns named tracing scopes, default enable/disable policy, subscriber bootstrap and
  buffered dump-on-warn/error behavior
- **Icons registry** — codegen'd Rust icon access backed by `slint-adapter/ui/assets`

### PageStatus

```rust
pub enum PageStatus { Inactive, Loading, Ready, Error }
```

Page state is identified by `(TabId, PageId)`. Features write to the registry via `report_page` / `report_tab` — the
registry decides whether to publish to the bus.

### ReactiveSetting

Binds a JSON store path to a `Signal<T>`. When the store changes, the signal updates and subscribers are notified.
Calling `.set()` writes back to the store.

### Tracing

Tracing policy and scope naming live in `context`, not in `desktop` and not in feature crates.

- Scope ids are stable dot-separated names: `ui.services.action`, `context.settings.save`, `core.bus.publish`
- The scope catalog source of truth is `crates/context/trace-scopes.toml`
- `crates/context/build.rs` codegens the scope catalog consumed by `context::trace`
- `context::trace::init_subscriber(...)` is the only supported tracing bootstrap entry point
- `desktop` may only provide sinks/writers (for example rolling log files); it should not own tracing policy
- Scope default on/off lives in `crates/context/trace-scopes.toml` boolean entries (`true`/`false`)
- `crates/context/trace-scopes.toml` may also contain `[policy]` arrays for default noisy-message / noisy-target
  suppression; use that instead of hardcoding trace filters in feature code
- Runtime overrides come from settings via `TraceSettingsFeature` and are resolved by prefix
- Do not add env-based trace overrides
- Low-level trace/debug/info history is buffered and dumped when the same correlation/op flow emits warn/error

Business/UI correlation rules:

- Business correlation is born in UI adapter callbacks, not in domain actors
- `#[ui_action(scope = \"ui.services.action\", ...)]` is the preferred way to create a correlated UI scope
- `app_core` only carries correlation/runtime metadata through `send`, `publish` and `spawn_bg`
- Domain code should reuse the current correlation id for external request/response protocols when one exists
- Do not thread `correlation_id` manually through every internal actor message unless the protocol truly requires it

---

## widgets

Only shared table code. Features write their own adapters to it. `widgets` has no knowledge of `contract` or
`slint-adapter`.

---

## contract

The layer between domain and UI. Contains **only**:

1. **Port traits** — commands from domain to UI (almost always unidirectional)
2. **Bindings traits** — callbacks from UI to domain
3. **DTOs** — data structures implementing `Message` for the event bus

Example of the split:

```rust
// Domain drives UI
pub trait ContextMenuUiPort: 'static {
    fn set_menu_open(&self, is_open: bool);
    fn show_menu(&self, x: f32, y: f32, reveal_delay_ms: u64);
    fn hide_menu(&self);
}

// UI notifies domain of user actions
pub trait ContextMenuUiBindings: 'static {
    fn on_show_context_menu<F>(&self, handler: F) where
        F: Fn(f32, f32) + 'static;
    fn on_close_menu<F>(&self, handler: F) where
        F: Fn() + 'static;
}
```

DTOs in `contract` may implement `Message` and be used by other features via the bus:

```rust
#[derive(Clone)]
pub struct RemoteScanResult {
    /* ... */
}
impl Message for RemoteScanResult {}
```

Data flow through `contract` is **predominantly unidirectional**. Domain writes to Port, UI reports via Bindings.
Bidirectional flow is a rare exception.

---

## domain

All features live here. Heavy features are extracted into separate crates (e.g. `domain_agents`, `domain_processes`,
`domain_environments`).

Features may use: `core`, `context`, `widgets`, and reference `contract`. They have no knowledge of `slint-adapter`.

### How a feature is structured

A feature is not a single file. It is a decomposition for a specific concern. The structure is free-form — the key is
separation of responsibilities. At minimum:

**1. Installation** (`Feature<TWindow>::install`) — bootstrap:

- create actor(s) and register them with `Reactor`
- subscribe to the event bus
- bind UI callbacks via `Bindings`
- set initial UI state via `Port`
- inject dependencies into `SharedState`

**2. Actor** — all business logic lives here. Receives the adapter (`Port`) on construction, communicates with UI via
messages. Subscribes to the bus to communicate with other features.

Actor messages are defined with the `messages!` macro:

```rust
messages! {
    Sort(SharedString),
    ViewportChanged { start: usize, count: usize },
    Select { pid: u32, idx: usize },
}
```

Each message gets its own `Handler<M, TWindow>` impl on the actor:

```rust
impl<P, TWindow> Handler<Sort, TWindow> for ProcessActor<P>
where
    P: ProcessesUiPort,
    TWindow: Window,
{
    fn handle(&mut self, msg: Sort, _ctx: &Context<Self, TWindow>) {
        self.table.toggle_sort(msg.0.clone());
        self.ui_port.set_sort_state(msg.0, self.table.sort_state().descending);
        self.push_batch();
    }
}
```

For async work, use `ctx.spawn_bg`:

```rust
fn handle(&mut self, _: TerminateSelected, ctx: &Context<Self, TWindow>) {
    ctx.spawn_bg(async move {
        // runs off the UI thread
        NoOp // return NoOp if no message should be sent back
    });
}
```

**3. Adapter** — injected into the feature from outside (via `with_adapter!`), passed into the actor. Implements the
`Port` trait from `contract`.

### Feature settings

If a feature has settings, they go in a dedicated `settings.rs`. Use the `#[feature_settings]` macro:

```rust
#[feature_settings(prefix = "process")]
pub struct ProcessSettings {
    #[setting(default = 1500u64)]
    scan_interval_ms: u64,

    #[setting(nested)]
    columns: ColumnsSettings,
}
```

- `prefix` — path in the JSON settings store
- `#[setting(default = ...)]` — default value, may be `serde_json::json!(...)`
- `#[setting(nested)]` — nested settings struct, also annotated with `#[feature_settings]` (without prefix)
- `DashMap<K, V>` — valid field type, default set via `serde_json::json!(...)`

Reference implementations: `domain/src/features/processes/settings.rs`, `domain/src/features/services/settings.rs`.

The macro generates:

- `ReactiveSetting<T>` fields for each `#[setting]`
- `Arc<NestedSettings>` fields for each `#[setting(nested)]`
- `::new(shared: &SharedState) -> anyhow::Result<Self>` — construction with automatic `ensure_defaults`
- getters `field_name() -> ReactiveSetting<T>` (cheap clone)
- getters `field_name() -> Arc<NestedSettings>` for nested structs
- setters `set_field_name(value: T) -> anyhow::Result<()>` — write to the store
- `store() -> &Arc<SettingsStore>` — direct store access

Usage in `install`:

```rust
let settings = ProcessSettings::new(shared) ?;
let interval = settings.scan_interval_ms(); // ReactiveSetting<u64>
interval.subscribe( | ms| { /* react to change */ });
```

### Feature examples

Minimal feature (SharedState injection only):

```rust
impl<TWindow: Window> Feature<TWindow> for PageStatusFeature {
    fn install(self, _reactor: &mut Reactor, _ui: &TWindow, shared: &SharedState) -> anyhow::Result<()> {
        let registry = Arc::new(PageStatusRegistry::new());
        shared.insert_arc(registry);
        Ok(())
    }
}
```

Feature with an adapter:

```rust
impl<TWindow, F, P> Feature<TWindow> for L10nFeature<F>
where
    F: Fn(&TWindow) -> P + 'static,
    P: L10nPort
{
    fn install(self, _: &mut Reactor, ui: &TWindow, _: &SharedState) -> anyhow::Result<()> {
        let port = (self.make_port)(ui);
        L10nManager::apply_to_port(&port);
        Ok(())
    }
}
```

---

## AgentsFeature

Lives in the `domain_agents` crate. The only point of contact with the outside world.

**External agents:**

- `windows` — runs on Windows, data source and command executor
- `wsl` — runs on WSL, data source and command executor

Communication is bidirectional: agents push data (reports), the feature sends commands (requests with `correlation_id`
for response matching).

Other features **do not communicate with agents directly** — only via the event bus, with `AgentsFeature` as the
intermediary. The protocol is encapsulated within the crate.

DTOs for bus communication are defined in `contract`:

```rust
pub struct WindowsReportMessage(pub WindowsReport);           // agent → domain
pub struct WindowsActionRequest {
    correlation_id: Uuid,
    request: WindowsRequest
}    // domain → agent
pub struct WindowsActionResponse {
    correlation_id: Uuid,
    response: WindowsResponse
} // agent → domain
```

---

## slint-adapter

Implementations of `contract` traits for Slint. Each adapter holds a `slint::Weak<AppWindow>` and implements `Port` +
`Bindings` via the Slint API.

### UI callback tracing

UI-originated actions should create correlation scopes via the `ui_adapter` macro support:

```rust
#[ui_action(scope = "ui.services.action", target = "name,kind")]
fn on_service_action<F>(&self, ui: &AppWindow, handler: F)
where
    F: Fn(SharedString, ServiceActionKind) + 'static;
```

Rules:

- Use dot-separated scope ids, never Rust module paths
- Put product/UI scopes in `crates/context/trace-scopes.toml`
- If a trace path is structurally useful but a few messages/targets are spammy, suppress them via `[policy]` in
  `crates/context/trace-scopes.toml` or the trace settings feature, not by deleting the whole scope
- Prefer `#[ui_action(...)]` over manual `in_ui_action_scope(...)` wrappers
- Noisy callbacks may be default-disabled in the scope catalog instead of inventing one-off logging logic

### Icons

Icons live in `slint-adapter/ui/assets/`. To add a new icon:

1. Add a line to `slint-adapter/ui/assets/download.txt` in the format `name:url`:

```
apps-list:https://api.iconify.design/fluent-color:apps-list-24.svg
dismiss:https://api.iconify.design/fluent:dismiss-20-regular.svg
```

2. The build script watches this file and downloads icons into the same folder automatically.

Nothing else is needed — `context::icons::Icons::get("name")` access is codegen'd in `crates/context/src/icons.rs`.

---

## slint-adapter/ui (Slint UI)

Language: Slint. Directory structure:

| Path                   | Contents                                                                                      |
|------------------------|-----------------------------------------------------------------------------------------------|
| `assets/`              | SVG icons. Do not add manually — see the icons section above.                                 |
| `builtin/`             | The current dashboard (`BuiltinDashboard`).                                                   |
| `components/`          | Reusable components in Fluent Design style (Microsoft). Key trait: transparency.              |
| `content/`             | Dashboard container.                                                                          |
| `features/`            | 1-to-1 mapping to features from `domain`.                                                     |
| `pages/`               | Dashboard pages. At this stage features cover the FSD need; pages are used inside `builtin/`. |
| `shared/`              | Theme, locales, icons. Locales and icons are codegen'd — do not edit.                         |
| `app-window.slint`     | Root window. Tracks width and proxies breakpoints (`sm` / `md` / `lg`) into `WindowAdapter`.  |
| `globals-export.slint` | Re-exports everything the Rust side needs. When adding a new global, add it here.             |
| `window-adapter.slint` | Window resize adapter implementation.                                                         |

### features/

Each feature is a folder with a mandatory `index.slint` (entry point, everything re-exported from here) and
`globals.slint` (feature state).

`globals.slint` contains `export global FeatureNameGlobal` with:

- `in property` — data from Rust to UI
- `in-out property` — bidirectional (e.g. column widths)
- `callback` — events from UI to Rust

Example:

```slint
export global ServicesFeatureGlobal {
    in property <[ServiceEntry]> service-rows: [];
    in-out property <[TableColWidth]> column-widths: [...];
    callback sort-by(string);
    callback select-service(string, int);
}
```

When adding a new global, re-export it from `globals-export.slint`.

### Conventions

- New component in `components/` — Fluent Design, transparent background
- New feature in `features/` — folder with `index.slint` + `globals.slint`, re-export from `globals-export.slint`
- Icons — only via `download.txt`, never place SVGs manually
- Locales and codegen'd icons in `shared/` — do not edit

---

## desktop

Entry point. Aggregation only — no logic.

- `main.rs` should stay thin and delegate startup into a dedicated bootstrap module
- `desktop` may create log directories / rolling file appenders and pass them into `context::trace`
- Feature installation still happens here, in the required order, before `app.run()`

Installation order matters: features that insert into `SharedState` must come before features that read from it.
