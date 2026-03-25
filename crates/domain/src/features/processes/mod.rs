use app_core::app::{Feature, FromUiWeak};
use app_core::reactor::Reactor;

use crate::features::processes::application::actors::*;
use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::ColumnWidthConfig;
use app_contracts::features::agents::{RemoteScanResult, ScanTick};
#[cfg(target_os = "windows")]
use app_contracts::features::agents::WindowsReportMessage;
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::{ProcessesUiBindings, ProcessesUiPort};
use app_core::SharedState;
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EVENT_BUS;
use app_core::settings::{FeatureSettings, SettingsScope, SettingsStore, settings_from};
use app_core::windowed_rows::WindowedRows;
use serde_json::Value;
use slint::ComponentHandle;
use std::collections::HashMap;
use std::time::Duration;

mod application;
mod domain;
mod scanner;
mod services;
pub mod ui;

const SCAN_INTERVAL_MS: &str = "scan_interval_ms";
const METADATA_NAME_CACHE_TTL_SECS: &str = "metadata.name_cache_ttl_secs";
const METADATA_ICON_CACHE_TTL_SECS: &str = "metadata.icon_cache_ttl_secs";
const COLUMNS_DEFAULT_WIDTH_PX: &str = "columns.default_width_px";
const COLUMNS_WIDTHS_PX: &str = "columns.widths_px";
const COLUMNS_MIN_WIDTHS_PX: &str = "columns.min_widths_px";

struct ProcessSettings;

impl SettingsScope for ProcessSettings {
    const PREFIX: &'static str = "process";
}

impl FeatureSettings for ProcessSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, SCAN_INTERVAL_MS, 1500u64)?;
        Self::ensure_default(settings, METADATA_NAME_CACHE_TTL_SECS, 300u64)?;
        Self::ensure_default(settings, METADATA_ICON_CACHE_TTL_SECS, 120u64)?;
        Self::ensure_default(settings, COLUMNS_DEFAULT_WIDTH_PX, 70u64)?;
        Self::ensure_default(settings, COLUMNS_WIDTHS_PX, serde_json::json!({}))?;
        Self::ensure_default(settings, COLUMNS_MIN_WIDTHS_PX, serde_json::json!({}))?;
        Ok(())
    }
}

pub struct ProcessFeature<F> {
    pub show_icons: bool,
    make_ui_port: F,
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessFeatureBuilder {
    show_icons: bool,
}

impl<F> ProcessFeature<F> {
    pub fn new(show_icons: bool, make_ui_port: F) -> Self {
        Self {
            show_icons,
            make_ui_port,
        }
    }
}

impl ProcessFeature<()> {
    pub fn builder() -> ProcessFeatureBuilder {
        ProcessFeatureBuilder { show_icons: true }
    }
}

impl ProcessFeatureBuilder {
    pub fn show_icons(mut self, show_icons: bool) -> Self {
        self.show_icons = show_icons;
        self
    }

    pub fn with_ui_port<F>(self, make_ui_port: F) -> ProcessFeature<F> {
        ProcessFeature::new(self.show_icons, make_ui_port)
    }

    pub fn with_adapter<TWindow, TAdapter>(
        self,
    ) -> ProcessFeature<impl Fn(&TWindow) -> TAdapter + 'static>
    where
        TWindow: ComponentHandle + 'static,
        TAdapter: FromUiWeak<TWindow> + ProcessesUiPort + ProcessesUiBindings + Clone + 'static,
    {
        ProcessFeature::new(self.show_icons, |ui: &TWindow| {
            TAdapter::from_ui_weak(ui.as_weak())
        })
    }
}

impl<TWindow, F, P> Feature<TWindow> for ProcessFeature<F>
where
    TWindow: ComponentHandle + 'static,
    F: Fn(&TWindow) -> P + 'static,
    P: ProcessesUiPort + ProcessesUiBindings + Clone + 'static,
{
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);
        ProcessSettings::ensure_defaults(&settings)?;

        let scan_interval_ms = ProcessSettings::setting_or(&settings, SCAN_INTERVAL_MS, 1500u64)?;
        let name_cache_ttl_secs =
            ProcessSettings::setting_or(&settings, METADATA_NAME_CACHE_TTL_SECS, 300u64)?;
        let icon_cache_ttl_secs =
            ProcessSettings::setting_or(&settings, METADATA_ICON_CACHE_TTL_SECS, 120u64)?;
        let default_width_px = ProcessSettings::setting_or(&settings, COLUMNS_DEFAULT_WIDTH_PX, 70u64)?;
        let widths_px =
            ProcessSettings::setting_or(&settings, COLUMNS_WIDTHS_PX, serde_json::json!({}))?;
        let min_widths_px =
            ProcessSettings::setting_or(&settings, COLUMNS_MIN_WIDTHS_PX, serde_json::json!({}))?;
        let width_config = load_width_config(
            default_width_px.get(),
            widths_px.get_arc().as_ref(),
            min_widths_px.get_arc().as_ref(),
        );
        let ui_port = (self.make_ui_port)(ui);

        let process_actor = ProcessActor {
            flow: ProcessFlowState::new(self.show_icons),
            metadata: ProcessMetadataService::new(
                Duration::from_secs(name_cache_ttl_secs.get().max(1)),
                Duration::from_secs(icon_cache_ttl_secs.get().max(1)),
            ),
            widths_by_schema: HashMap::new(),
            width_config,
            name_cache_ttl_secs,
            icon_cache_ttl_secs,
            default_width_px,
            widths_px,
            min_widths_px,
            is_active: true,
            ui_port: ui_port.clone(),
            rows_window: WindowedRows::new(50),
            snapshots: Default::default(),
        };

        let addr = Addr::new(process_actor, ui.as_weak());

        let a = addr.clone();
        ui_port.on_sort_by(move |field| a.send(Sort(field)));
        let a = addr.clone();
        ui_port.on_toggle_expand_group(move |group| a.send(ToggleExpand(group)));
        ui_port.on_terminate(addr.handler(TerminateSelected));
        let a = addr.clone();
        ui_port.on_select_process(move |pid, idx| {
            a.send(Select {
                pid: pid as u32,
                idx: idx as usize,
            })
        });
        let a = addr.clone();
        ui_port.on_rows_viewport_changed(move |start, count| {
            a.send(ViewportChanged {
                start: start.max(0) as usize,
                count: count.max(0) as usize,
            })
        });

        EVENT_BUS.with(|bus| {
            bus.subscribe::<ProcessActor<P>, TabChanged, TWindow>(addr.clone());
            bus.subscribe::<ProcessActor<P>, RemoteScanResult, TWindow>(addr.clone());
            #[cfg(target_os = "windows")]
            bus.subscribe::<ProcessActor<P>, WindowsReportMessage, TWindow>(addr.clone());
        });

        reactor.add_dynamic_loop(
            move || Duration::from_millis(scan_interval_ms.get().max(1)),
            || EVENT_BUS.with(|bus| bus.publish(ScanTick)),
        );

        Ok(())
    }
}

fn load_width_config(
    default_width_px: u64,
    widths_px: &Value,
    min_widths_px: &Value,
) -> ColumnWidthConfig {
    let mut cfg = ColumnWidthConfig::default();
    let default_width = default_width_px.clamp(40, 1000) as u32;
    cfg.default_width_px = default_width;
    cfg.widths_px = parse_width_map(widths_px);
    cfg.min_widths_px = parse_width_map(min_widths_px);
    cfg
}

fn parse_width_map(value: &Value) -> HashMap<String, u32> {
    let mut out = HashMap::new();
    let Some(map) = value.as_object() else {
        return out;
    };

    for (key, value) in map {
        let Some(raw) = value.as_u64() else {
            continue;
        };
        out.insert(key.clone(), raw.clamp(40, 1000) as u32);
    }

    out
}
