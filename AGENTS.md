# AGENTS.md

Architecture guide for AI agents. Read this before touching any code.

---

## TL;DR

- Business logic lives in **Managed Actors** inside `domain`
- Features communicate via **`EventBus`** (Pub/Sub) or **`AsyncBus`** (RPC) — never call each other directly
- UI knows nothing about `domain` — only about `contract` (Port + Bindings traits)
- **Wiring**: Use generated **Binders** to connect UI callbacks to actor messages. Manual closures in `install()` are
  forbidden.
- **Actor Manifest**: Use `#[actor_manifest]` to declare bus subscriptions and handlers. Manual `EventBus::subscribe` in
  `install()` is forbidden.
- **Zero Generics**: Do not use `<TWindow>` in domain logic; use `FeatureContextState`
- Navigation is **URI-based** (e.g., `host://processes`) and driven by the framework Route Registry
- Before build/check/run commands, inspect `.cargo/*.toml` for project aliases and toolchain config
- Prefer project cargo aliases for verification; default check command is `cargo cdev`
- When in doubt, copy `processes` or `services` as a reference implementation

---

## Never edit manually (Codegen)

- `crates/app-contracts/src/capabilities.rs` — generated from `#[capability]`
- `slint-adapter/ui/shared/capabilities.slint` — Slint properties for capability matching
- `crates/context/src/icons.rs` — codegen'd icon registry from `slint-adapter/ui/assets`
- `crates/domain-test-kit/src/generated.rs` — UI stubs for testing
- `crates/domain/src/features/l10n/apply.rs` — codegen'd by `crates/domain/build.rs` from
  `crates/domain/locales/en.toml`
- `crates/context/src/trace.rs` scope catalog section — codegen'd from `crates/context/trace-scopes.toml`
- `slint-adapter/ui/shared/localization.slint` — codegen'd by `crates/slint-adapter/build.rs` from
  `crates/domain/locales/en.toml`
- `slint-adapter/ui/shared/icons.slint` — codegen'd from `download.txt`
- `slint-adapter/ui/globals-export.slint` — codegen'd by `build.rs` (scans `ui/` for exported structs, enums, globals
  and windows)
- `crates/app-contracts/contracts-schema.json` — metadata used for macro expansion

---

## What is this project

A task manager replacement. Rust, UI built with Slint.

Longer-term, treat it as a hub for system observability tools rather than only a task manager clone.

---

## Architecture vocabulary

Use these labels as shorthand for the current design. They are descriptive, not dogma.

- **Managed Actor** — An actor declaring its subscriptions (`bus!`) and handlers (`handlers!`) via `#[actor_manifest]`.
  The only valid actor form in domain.
- **AppUri** — Addressing system for navigation (e.g., `host://processes?capabilities=feat.list`)
- **Binder** — A generated typed bridge connecting UI `Port` callbacks to Actor messages. Replaces manual closures in
  `install()`
- **Framework** — The heavyweight infrastructure layer: Reactor, App lifecycle, URI Routing, Settings, Native OS
- **Stabilizer** — A testing utility that ensures all async bus messages are processed before assertions
- **Ports & Adapters / Hexagonal** — `domain` does not know Slint; UI integration lives behind `contract` traits and
  `slint-adapter`
- **Actor-based application layer** — features organize behavior around actors, messages and local actor state
- **Event-driven feature communication** — feature-to-feature interaction goes through `EventBus`/`AsyncBus`, never
  direct calls
- **CQRS-style UI flow** — UI sends intents/commands via `Bindings`; domain pushes read/view state back via `Port`
- **Context as environment/infrastructure** — `context` owns caches, icons, locales and other resource/runtime concerns,
  not business logic
- **UI-origin correlation** — user-initiated tracing/correlation starts in UI adapter callbacks; `app-core` only
  propagates it through runtime hops

---

## Crate structure

| Crate             | Role                                                                        | Depends on                         |
|-------------------|-----------------------------------------------------------------------------|------------------------------------|
| `desktop`         | Entry point, feature aggregation                                            | Everything                         |
| `framework`       | **Infrastructure Hub**: Reactor, App, Routing, Settings, Tracing, Native OS | `app-core`                         |
| `app-core`        | **Messaging Foundation**: EventBus, AsyncBus, Signals, Actor traits         | —                                  |
| `slint-adapter`   | Slint-specific implementations of contracts                                 | `contract`, `framework`            |
| `contract`        | Port + Bindings traits, DTOs, Capabilities                                  | `framework`                        |
| `domain/*`        | Business logic (Managed Actors), may be separate crates                     | `framework`, `context`, `contract` |
| `domain-test-kit` | Test harness, auto-stubs, and Stabilizers                                   | `domain`, `framework`              |
| `context`         | **Environment Data**: caches, locales, icons                                | `framework`                        |
| `widgets`         | Shared UI code (tables)                                                     | `app-core`                         |
| `build-utils`     | Shared build-time helpers for codegen/build scripts                         | —                                  |

`context` and `widgets` have no knowledge of `slint-adapter` or `contract`. `slint-adapter` has no knowledge of
`domain`.

---

## Rules

- **Managed Actors only**: Always use `#[actor_manifest]`. Manual `EventBus::subscribe` in `install()` is forbidden.
- **No Direct Calls**: Features must use `EventBus::publish` or `AsyncBus::request`.
- **Zero Generics**: Business logic must not depend on `<TWindow>`. Use `FeatureContextState`.
- **URI-First**: Navigation must go through URI segments. Hardcoded page IDs are obsolete.
- **Binders over closures**: Use generated Binders in `WindowFeature::install()`. Manual
  `port.on_click(move || addr.send(Msg))` is forbidden.
- In `crates/app-contracts/src/features/<feature>/`, split contracts by role only when it adds clarity: use `model.rs`,
  `bindings.rs`, `ports.rs` selectively, avoid empty placeholder files, keep `mod.rs` re-exporting the public API
- New feature = use an existing feature as the reference implementation
- Heavy feature = separate crate
- Do not add `contract` or `slint-adapter` dependencies to `context` or `widgets`
- Do not communicate with agents bypassing `AgentsFeature`
- `SharedState` is for bootstrap only, not business logic
- Do not invent new tracing conventions ad hoc — use the scope/correlation model described below
- Platform-dependent code must live in a dedicated subfolder/module, with one file per platform (`windows.rs`,
  `linux.rs`, `macos.rs`) side-by-side. `mod.rs` must be a thin proxy. Do not scatter `#[cfg(...)]` branches across
  unrelated files.

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
- Coverage: `cargo cov`

**Adding a feature**

1. Define traits in `app-contracts`
2. Implement logic in `domain` using a `ManagedActor`
3. Implement `WindowFeature` and wire UI via a `Binder`
4. Register in `desktop/src/bootstrap.rs`

**Adding a setting**
Add a field to `settings.rs` with `#[setting(default = ...)]`. Use the generated getter in the actor.
Reference: `domain/src/features/processes/settings.rs`.

**Adding an icon**
Add a line to `slint-adapter/ui/assets/download.txt` in the format `name:url`, rebuild. Access via
`context::icons::Icons::get("name")` in Rust or the codegen'd Slint binding.

**Adding a locale string**
Edit `crates/domain/locales/*.toml`, rebuild. Do not touch any generated files.

**Adding a trace scope**
Edit `crates/context/trace-scopes.toml`, rebuild. Do not hand-edit the generated scope catalog in
`crates/context/src/trace.rs`.

**Adding a UI feature**
Create `slint-adapter/ui/features/my-feature/` with `index.slint` + `globals.slint`. Re-export the global from
`globals-export.slint`.

**Publishing a bus message**

```rust
EventBus::publish(MyMessage { ... });
```

**Subscribing (Managed Actor)**

```rust
#[actor_manifest]
impl ManagedActor for MyActor {
    type Bus = bus!(LocalMsg, @ExternalMessage); // @ = EventBus
    type Handlers = handlers!(LocalMsg, @ExternalMessage, DoWork);
}
```

**RPC between features**

```rust
let response = AsyncBus::request::<MyRequest, MyResponse>(MyRequest { ... }).await;
```

---

## app-core

The messaging foundation.

- **`EventBus`** — high-performance Pub/Sub
- **`AsyncBus`** — RPC support via `request<Req, Res>(...)`
- **`Signal<T>`** — reactive primitive, subscribe to value changes
- **`UiThreadToken`** — proof of UI-thread access
- **Actor traits** — base traits consumed by `framework`

---

## framework

The heavyweight infrastructure core. Owns everything that was previously split between `core` and parts of `context`.

- **`Reactor`** — manages actor runtimes and background loops
- **`App<TWindow>`** — the application container
- **`RouteStatusRegistry`** — registry of URI health (Loading / Ready / Error)
- **`SettingsStore`** — persistent JSON storage
- **`ReactiveSetting<T>`** — a setting backed by `Signal<T>`: reads from the store, reacts to changes, writes back
- **`FeatureComponent`** — trait for actors to handle activation/deactivation based on URI

### Feature traits

```rust
pub trait AppFeature {
    fn install(self, ctx: &mut AppFeatureInitContext) -> anyhow::Result<()>;
}

pub trait WindowFeature<TWindow: Window> {
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()>;
}
```

### App assembly

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

Installation order matters: features that insert into `SharedState` must come before features that read from it.

---

## context

The environment features operate in. Not UI, not business logic — environment. Depends on `framework`.

Contains:

- **String caches** — buffers between UI and domain for string data
- **Icon cache** — extracts icons from processes
- **Locales** — `crates/domain/locales/*.toml` is the source of truth. `crates/domain/build.rs` generates
  `crates/domain/src/features/l10n/apply.rs`, and `crates/slint-adapter/build.rs` generates
  `slint-adapter/ui/shared/localization.slint`. Add/edit strings only in the `.toml` files.
- **Trace catalog + policy** — owns named tracing scopes, default enable/disable policy, subscriber bootstrap and
  buffered dump-on-warn/error behavior
- **Icons registry** — codegen'd Rust icon access backed by `slint-adapter/ui/assets`

### Tracing

Tracing policy and scope naming live in `context`, not in `desktop` and not in feature crates.

- Scope ids are stable dot-separated names: `ui.services.action`, `context.settings.save`, `core.bus.publish`
- The scope catalog source of truth is `crates/context/trace-scopes.toml`
- `crates/context/build.rs` codegens the scope catalog consumed by `context::trace`
- `context::trace::init_subscriber(...)` is the only supported tracing bootstrap entry point
- `desktop` may only provide sinks/writers (e.g., rolling log files); it should not own tracing policy
- Scope default on/off lives in `crates/context/trace-scopes.toml` boolean entries (`true`/`false`)
- `crates/context/trace-scopes.toml` may also contain `[policy]` arrays for default noisy-message / noisy-target
  suppression; use that instead of hardcoding trace filters in feature code
- Runtime overrides come from settings via `TraceSettingsFeature` and are resolved by prefix
- Do not add env-based trace overrides
- Low-level trace/debug/info history is buffered and dumped when the same correlation/op flow emits warn/error

Business/UI correlation rules:

- Business correlation is born in UI adapter callbacks, not in domain actors
- `#[slint_bindings]` methods are the source of truth for UI-action tracing metadata; use `#[tracing(target = "...")]`
  on the contract method when target field names need an explicit override
- `slint_bindings_adapter` derives the UI-action scope automatically as `Ui.<Feature>.<method_name>` (`Ui` prefix is
  removed from the trait name before building the feature segment)
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
4. **Capabilities** — feature access control via `#[capability(...)]`

Feature contracts live under `crates/app-contracts/src/features/<feature>/` and may be split by concern when that
actually improves readability. Do not force 1:1 files just for symmetry.

- `model.rs` — DTOs, VMs, enums, constants, bus messages, helper types
- `bindings.rs` — `Ui...Bindings` traits, when a feature actually has bindings
- `ports.rs` — `Ui...Port` traits and adjacent adapter-facing contracts, when they exist
- `mod.rs` — internal module wiring plus public re-exports so external imports stay stable
- If a feature only has one concern, keep it compact instead of introducing empty modules

### Capabilities

```rust
#[capability("processes.list")]
pub struct ProcessesCapability;
```

### RPC bindings

```rust
rpc_bind!(MyRequest => MyResponse);
```

### Port / Bindings example

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

## domain: Managed Actors

All features live here. Heavy features are extracted into separate crates (e.g. `domain_agents`, `domain_processes`,
`domain_environments`).

Features may use: `framework`, `app-core`, `context`, `widgets`, and reference `contract`. They have no knowledge of
`slint-adapter`.

### Structure

Every actor implements `ManagedActor` and `FeatureComponent`. `<TWindow>` is banned from domain.

```rust
#[actor_manifest]
impl ManagedActor for MyActor {
    type Bus = bus!(LocalMsg, @RouteActivated);
    type Handlers = handlers!(LocalMsg, @RouteActivated, DoWork);
}

pub struct MyActor {
    ctx: FeatureContextState, // ties actor to window and capability
}

impl FeatureComponent for MyActor {
    fn context_state(&mut self) -> &mut FeatureContextState { &mut self.ctx }
    fn on_activated(&mut self, uri: &AppUri, ctx: &Context<Self>) { ... }
}
```

### Handlers

Use `#[handler]`. Background work uses `ctx.spawn_bg`:

```rust
#[handler]
fn sort_table<P: UiProcessesPort>(this: &mut ProcessActor<P>, msg: Sort) {
    this.table.toggle_sort(msg.0.clone());
    this.ui_port.set_sort_state(msg.0, this.table.sort_state().descending);
    this.push_batch();
}

#[handler]
fn terminate_process<P: UiProcessesPort>(
    this: &mut ProcessActor<P>,
    _: TerminateSelected,
    ctx: &Context<ProcessActor<P>>
) {
    ctx.spawn_bg(async move {
        // runs off the UI thread
        NoOp // return NoOp if no message should be sent back
    });
}
```

Actor messages are defined with the `messages!` macro:

```rust
messages! {
    Sort(SharedString),
    ViewportChanged { start: usize, count: usize },
    Select { pid: u32, idx: usize },
}
```

### Feature settings

Use the `#[feature_settings]` macro in a dedicated `settings.rs`:

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

The macro generates: `ReactiveSetting<T>` fields, `Arc<NestedSettings>` for nested, `::new(shared: &SharedState)`,
getters, setters, and `store()`.

Reference implementations: `domain/src/features/processes/settings.rs`, `domain/src/features/services/settings.rs`.

---

## slint-adapter & Binders

`slint-adapter` contains Slint implementations of `contract` traits. Each adapter holds a `slint::Weak<AppWindow>` and
implements `Port` + `Bindings` via the Slint API.

**Why this crate exists:** Compilation firewall. Slint macro codegen is heavy; keeping it trapped here means tweaking a
`.slint` file never triggers recompilation of business logic.

### Binder pattern

Binders are generated typed bridges. Use them in `WindowFeature::install()` instead of manual closures:

```rust
fn install(&mut self, ctx: &mut WindowFeatureInitContext<T>) -> anyhow::Result<()> {
    let ui_port = (self.make_port)(ctx.ui);
    let addr = Addr::new_managed(MyActor::new(...), ctx.ui.new_token(), &self.tracker);

    MyBinder::new(&addr, &ui_port)
        .on_action_clicked(DoWork)
        .on_text_changed(|v| UpdateText(v.into()));
    Ok(())
}
```

### UI callback tracing

Contract layer owns tracing metadata; adapters consume it via `#[slint_bindings_adapter(...)]`:

```rust
#[slint_bindings(global = "ProcessesFeatureGlobal")]
pub trait ProcessesUiBindings: 'static {
    #[tracing(target = "pid,idx")]
    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static;
}
```

### Contract/adapter macros playbook

1. **Contract traits are the source of truth** — declare in `crates/app-contracts/src/features/<feature>/` with:
    - `#[slint_port(global = "...")]` for domain → UI calls
    - `#[slint_bindings(global = "...")]` for UI → domain callbacks
    - `#[slint_dto]` for DTO structs/enums captured in schema
    - Helper attributes: `#[manual]`, `#[slint(name = "...", global = "...")]`, `#[tracing(target = "...")]` /
      `#[tracing(skip)]`

2. **Schema is generated from contracts** — `crates/app-contracts/build.rs` runs `build_utils::collector::main()` and
   writes `contracts-schema.json`

3. **Adapters consume schema via proc macros** — use `#[slint_port_adapter(window = AppWindow)]` /
   `#[slint_bindings_adapter(window = AppWindow)]`; macros auto-generate missing impl methods from schema; existing
   methods are not overwritten

4. **Transform injected by macros**:
    - Removes explicit `ui: &AppWindow` from the final adapter method signature
    - Upgrades `self.ui` internally; panics on dropped weak handle
    - Port methods: emits `ui.adapter.call` debug traces
    - Bindings methods: wraps handlers in `app_core::trace::in_ui_action_scope` with scope `Ui.<Feature>.<method>`

5. **Agent checklist when adding/changing callbacks or port methods**:
    - Update contract trait first (`app-contracts`)
    - Prefer macro-generated adapter methods; add explicit impl only for non-trivial mapping
    - Use `#[manual]` only when generation is not enough
    - Keep tracing metadata at contract layer, not ad hoc in adapter code
    - Rebuild so `contracts-schema.json` is refreshed before validating macro behavior

`Theme` is driven from Rust via `UiCosmeticsPort`: on Windows, the cosmetics feature pushes the full system accent
palette into the `Theme` global; on non-Windows platforms, a stub palette with sensible defaults is used.

Tracing rules:

- Use dot-separated scope ids, never Rust module paths
- Put product/UI scopes in `crates/context/trace-scopes.toml`
- If a trace path is structurally useful but a few messages/targets are spammy, suppress them via `[policy]` in
  `crates/context/trace-scopes.toml`, not by deleting the whole scope
- Prefer contract-level `#[tracing(...)]` metadata over manual `in_ui_action_scope(...)` wrappers in adapter impls
- Noisy callbacks may be default-disabled in the scope catalog instead of inventing one-off logging logic

### Icons

```
# slint-adapter/ui/assets/download.txt
apps-list:https://api.iconify.design/fluent-color:apps-list-24.svg
dismiss:https://api.iconify.design/fluent:dismiss-20-regular.svg
```

Nothing else is needed — `context::icons::Icons::get("name")` access is codegen'd in `crates/context/src/icons.rs`.

---

## slint-adapter/ui (Slint UI)

Language: Slint.

| Path                   | Contents                                                                                 |
|------------------------|------------------------------------------------------------------------------------------|
| `assets/`              | SVG icons. Do not add manually — see the icons section above.                            |
| `builtin/`             | The current dashboard (`BuiltinDashboard`).                                              |
| `components/`          | Reusable components in Fluent Design style (Microsoft). Key trait: transparency.         |
| `content/`             | Dashboard container.                                                                     |
| `features/`            | 1-to-1 mapping to features from `domain`.                                                |
| `pages/`               | Dashboard pages. Features cover the FSD need; pages are used inside `builtin/`.          |
| `shared/`              | Theme, locales, icons. Locales and icons are codegen'd — do not edit.                    |
| `app-window.slint`     | Root window. Tracks width and proxies breakpoints (`sm`/`md`/`lg`) into `WindowAdapter`. |
| `globals-export.slint` | Codegen'd re-exports of all entities (globals, structs, enums, windows).                 |
| `window-adapter.slint` | Window resize adapter implementation.                                                    |

### Routing

Routes are declared in `.slint` by exporting a `PageSpec` global. The framework scans these at startup to populate the
Route Registry:

```slint
export global MyPageSpec {
    out property <string> layout: "with-sidebar";
    out property <[string]> features: [Capabilities.processes-list];
}
```

### features/

Each feature is a folder with `index.slint` (entry point, everything re-exported from here) and `globals.slint`:

```slint
export global ServicesFeatureGlobal {
    in property <[ServiceEntry]> service-rows: [];
    in-out property <[TableColWidth]> column-widths: [...];
    callback sort-by(string);
    callback select-service(string, int);
}
```

(`in property` — data from Rust to UI; `in-out property` — bidirectional; `callback` — events from UI to Rust.)

Adding a new global/struct/enum/window in any `.slint` file with `export` will automatically include it in
`globals-export.slint` on next build.

### Conventions

- New component in `components/` — Fluent Design, transparent background
- New feature in `features/` — folder with `index.slint` + `globals.slint`
- Icons — only via `download.txt`, never place SVGs manually
- Locales and codegen'd icons in `shared/` — do not edit

---

## Testing (domain-test-kit)

Tests use auto-generated stubs and a `Stabilizer`. Every UI interaction in a test must be followed by
`.stabilize(&mut h)`.

```rust
#[rstest]
fn test_logic(mut h: FeatureHarness) {
    let stub = MyUiStub::new();
    h.install(MyFeature::new(move |_| stub.clone())).unwrap();

    stub.emit_click().stabilize(&mut h);
    assert_eq!(stub.set_value_call_count().stabilize(&mut h), 1);
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
pub struct WindowsReportMessage(pub WindowsReport);            // agent → domain
pub struct WindowsActionRequest {
    correlation_id: Uuid,
    request: WindowsRequest
}     // domain → agent
pub struct WindowsActionResponse {
    correlation_id: Uuid,
    response: WindowsResponse
}  // agent → domain
```

---

## desktop

Entry point. Aggregation only — no logic.

- `main.rs` should stay thin and delegate startup into a dedicated bootstrap module (`bootstrap.rs`)
- `desktop` may create log directories / rolling file appenders and pass them into `context::trace`
- Feature installation happens in `bootstrap.rs`, in the required order, before `app.run()`

Installation order matters: features that insert into `SharedState` must come before features that read from it.