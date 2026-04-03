# AGENTS.md

Architecture guide for AI agents. Read this before touching any code.

---

## TL;DR

- Business logic lives in **actors** inside `domain`
- Features communicate only via **`EventBus`** — never call each other directly
- UI knows nothing about `domain` — only about `contract` (Port + Bindings traits)
- `install()` is the only bootstrap entry point for a feature
- When in doubt, copy `processes` or `services` as a reference

---

## Never edit manually

- `core/src/icons.rs` — codegen'd icon registry
- `crates/context/src/l10n.rs` — codegen'd from `.toml`
- `slint-adapter/ui/shared/localization.slint` — codegen'd from `.toml`
- `slint-adapter/ui/shared/icons.slint` — codegen'd from `download.txt`

---

## What is this project

A task manager replacement. Rust, UI built with Slint.

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
- Tracing conventions are not yet established — do not invent them; use `#[instrument]` only where it already exists in
  the codebase

---

## Common tasks

**Adding a feature**
Create a folder in `domain/src/features/` (or a new crate for heavy features), implement `Feature<TWindow>`, register in
`desktop/src/main.rs`. Reference: `domain/src/features/services/`.

**Adding a setting**
Add a field to `settings.rs` with `#[setting(default = ...)]`, rebuild. Use the generated getter in the actor.
Reference: `domain/src/features/processes/settings.rs`.

**Adding an icon**
Add a line to `slint-adapter/ui/assets/download.txt` in the format `name:url`, rebuild. Access via `Icons::get("name")`
in Rust or the codegen'd Slint binding.

**Adding a locale string**
Edit `context/locales/*.toml`, rebuild. Do not touch any generated files.

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
- **`Icons::get(name)`** — codegen'd icon access (`Icons::get("apps-list")` → `Image`). See `core/src/icons.rs`. Do not
  edit manually.

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

### PageStatus

```rust
pub enum PageStatus { Inactive, Loading, Ready, Error }
```

Page state is identified by `(TabId, PageId)`. Features write to the registry via `report_page` / `report_tab` — the
registry decides whether to publish to the bus.

### ReactiveSetting

Binds a JSON store path to a `Signal<T>`. When the store changes, the signal updates and subscribers are notified.
Calling `.set()` writes back to the store.

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

### Icons

Icons live in `slint-adapter/ui/assets/`. To add a new icon:

1. Add a line to `slint-adapter/ui/assets/download.txt` in the format `name:url`:

```
apps-list:https://api.iconify.design/fluent-color:apps-list-24.svg
dismiss:https://api.iconify.design/fluent:dismiss-20-regular.svg
```

2. The build script watches this file and downloads icons into the same folder automatically.

Nothing else is needed — `Icons::get("name")` access is codegen'd in `core/src/icons.rs`.

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

Entry point. Aggregation only — no logic. Creates `AppWindow`, installs all features in the correct order, calls
`app.run()`.

Installation order matters: features that insert into `SharedState` must come before features that read from it.