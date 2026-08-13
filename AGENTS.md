# AGENTS.md

## What this is

`uniproc` is a system monitor for Windows 11 and WSL: processes, services, machine
metrics, WSL distributions. The UI is WinUI 3 driven from Rust through
`windows-reactor`, with `guinea` on top providing routing, features, reducers and
actors. Metrics are collected by out-of-process agents (a Windows service, and an
eBPF agent inside WSL) and arrive over capnp-rpc.

## Crates

| crate | role |
|---|---|
| `app-contracts` | Contracts shared by domain and UI: row types, messages, `#[port]`, `#[reducer]`, `#[actions]`. No logic, no platform calls. |
| `domain` | One module per feature: actor, scanner, settings, `install`. Owns all behaviour. |
| `ui` | Views. Pages, widgets, theme tokens, localization accessors. Reads reducer state, dispatches actions. |
| `context` | Encoding/extraction helpers and the icon cache. |
| `desktop` | Binary: routes, layouts, page wiring, app features, tracing. |
| `xtask` | Dev tasks (`cargo ragent`). |

Direction of dependencies: `desktop` → `ui` → `app-contracts` ← `domain`.
`ui` never depends on `domain`; they meet only at contracts.

## How a feature is assembled

Four files, one per layer:

1. `app-contracts/src/features/<f>/contracts.rs` — rows, `<F>Msg`, `#[port]`,
   `#[reducer]`, `#[actions]`.
2. `domain/src/features/<f>/actor.rs` — an actor with `#[handler]` functions and a
   `handlers!` list; scanning, sorting, selection.
3. `domain/src/features/<f>/install.rs` — spawns the actor, subscribes it to the
   global bus, binds actions to messages, owns any heartbeat in `ctx.scope`.
4. `ui/src/pages/<f>/` — the view.

`desktop/src/pages/<f>.rs` ties them: `Page::install` calls the domain installer,
`Page::view` calls the UI view. Routes are declared in `desktop/src/routes.rs`.

App-scoped features (those that outlive a page, like `agents`) implement
`AppFeature` and are installed in `desktop/src/main.rs` instead.

Anything long-lived — heartbeats, subscriptions — must be **owned**: `ctx.scope.own(..)`
for page scope, `ctx.tracker.track_loop(..)` for app scope. A dropped handle stops the
timer silently.

## Page layout

Every page has the same shape:

```
ui/src/pages/<name>/
  mod.rs          mod components; mod page; pub use page::<name>_view;
  page.rs         the view only
  components/     columns, overlays, cells
```

`components` is a private module: nothing from one page is reachable from another.

## Styles

Tokens live in `ui/src/theme/`: `space` (WinUI scale 4/8/12/16/24/36/48), `radius`,
`size`, and `palette` for the few brushes WinUI does not define.

Rules:

- Colours come from `windows_reactor::tokens` (`SystemSuccess`, `DividerStroke`,
  `LayerFill`, …). They resolve natively and follow theme and high contrast.
  Do not hardcode RGB for anything the system already names.
- Own brushes go through `Palette::of(cx.use_color_scheme())`, never as a bare
  constant — a constant has one value for both themes.
- Type comes from the ramp factories (`caption`, `body`, `body_strong`, `title`),
  not from `.font_size(..)`.
- A token is what must match **between** components. Arithmetic inside one component
  (`CHEVRON_SLOT_WIDTH`, `NAME_TEXT_INSET`) stays local.

Table cell styling is in `ui/src/widgets/table_cell.rs`; byte formatting is
`ui/src/format.rs` and is not a style.

## Localization

Strings live in `locales/en/`, mirroring the UI tree:

```
common.ftl              terms shared across surfaces
layouts/shell.ftl
pages/{processes,services,wsl}.ftl
widgets/metric-chart.ftl
```

Fluent ids are flat and global — the file is packaging, the **prefix** is the
namespace. Prefix by file name (`processes-col-name`). Do not share a string just
because two surfaces spell it the same in English.

Accessors are generated from the `.ftl` files. In components use `use_tr(cx)`, which
re-renders on language change; in free functions without a `cx` use `tr()` and let the
parent re-render.

Never use a localized string as a persistence key. `ProcessCategory` has `id()` for
storage and a `category_label(&l10n, ..)` for display, for exactly this reason.

## Never edit by hand

- `app-contracts/src/icons.rs` and the generated l10n accessors — produced by
  `app-contracts/build.rs` from `icons.gui.toml` and `locales/`.
- Anything under `target/*/out/`.
- `desktop/build.rs` generates app metadata from `app.toml`.

## Agents

`domain/features/agents` owns the connection: an actor per target, retry with a 5 s
cap, a scan heartbeat at app scope. Scan results arrive as `RemoteScanResult` on the
global bus; each feature subscribes to what it needs.

The Windows agent runs as a service in session 0, so it cannot enumerate user windows;
that is done in-process by `domain/features/processes/windows_scan.rs`.

## Testing and running

- `cargo test --workspace`.
- `cargo run` starts the app; it writes `run_desktop.log` next to the working directory
  as well as stderr, so a second instance does not overwrite the first one's log.
- `cargo ragent` runs the agent task from `xtask`.
- Trace scopes are configured in `trace-scopes.toml`.

## Code style

No comments in code — rationale belongs in commit messages and in this file.
