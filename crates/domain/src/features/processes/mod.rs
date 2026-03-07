use app_core::app::{Feature, FromUiWeak};
use app_core::reactor::Reactor;

use crate::features::processes::application::actors::*;
use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::scanner::wsl::{SharedWslClient, WslScanner};
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::ColumnWidthConfig;
use crate::features::settings::{settings_from, SettingsStore};
use crate::shared::settings::{FeatureSettings, SettingsScope};
use app_contracts::features::environments::WslAgentRuntimeEvent;
use app_contracts::features::navigation::TabChanged;
use app_contracts::features::processes::{ProcessesUiBindings, ProcessesUiPort};
use app_core::actor::addr::Addr;
use app_core::actor::event_bus::EVENT_BUS;
use app_core::SharedState;
use scanner::windows::WindowsScanner;
use slint::ComponentHandle;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
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
const COLUMNS_CPU_WIDTH_PX: &str = "columns.cpu.width_px";
const COLUMNS_MEMORY_WIDTH_PX: &str = "columns.memory.width_px";
const COLUMNS_MEMORY_MIN_WIDTH_PX: &str = "columns.memory.min_width_px";
const COLUMNS_DISK_WIDTH_PX: &str = "columns.disk_read.width_px";
const COLUMNS_NET_WIDTH_PX: &str = "columns.net.width_px";

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
        Self::ensure_default(settings, COLUMNS_CPU_WIDTH_PX, 70u64)?;
        Self::ensure_default(settings, COLUMNS_MEMORY_WIDTH_PX, 120u64)?;
        Self::ensure_default(settings, COLUMNS_MEMORY_MIN_WIDTH_PX, 120u64)?;
        Self::ensure_default(settings, COLUMNS_DISK_WIDTH_PX, 70u64)?;
        Self::ensure_default(settings, COLUMNS_NET_WIDTH_PX, 70u64)?;
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

        let scan_interval_ms = ProcessSettings::get_or(&settings, SCAN_INTERVAL_MS, 1500u64).max(1);

        let name_cache_ttl_secs =
            ProcessSettings::get_or(&settings, METADATA_NAME_CACHE_TTL_SECS, 300u64).max(1);

        let icon_cache_ttl_secs =
            ProcessSettings::get_or(&settings, METADATA_ICON_CACHE_TTL_SECS, 120u64).max(1);

        let width_config = load_width_config(&settings);

        let wsl_client: SharedWslClient = Arc::new(RwLock::new(None));
        let ui_port = (self.make_ui_port)(ui);

        let state = ProcessActor {
            flow: ProcessFlowState::new(self.show_icons),
            metadata: ProcessMetadataService::new(
                Duration::from_secs(name_cache_ttl_secs),
                Duration::from_secs(icon_cache_ttl_secs),
            ),
            scanners: Some(vec![
                Box::new(WindowsScanner::new()),
                Box::new(WslScanner::new(Arc::clone(&wsl_client))),
            ]),
            widths_by_schema: HashMap::new(),
            width_config,
            is_active: true,
            wsl_client,
            ui_port: ui_port.clone(),
        };

        let addr = Addr::new(state, ui.as_weak());

        let a = addr.clone();
        ui_port.on_sort_by(move |field| a.send(Sort(field)));
        let a = addr.clone();
        ui_port.on_toggle_expand_group(move |group| a.send(ToggleExpand(group)));
        ui_port.on_terminate(addr.handler(TerminateSelected));

        let addr_for_sub = addr.clone();
        let addr_for_wsl_sub = addr.clone();

        EVENT_BUS.with(|bus| {
            bus.subscribe::<ProcessActor<P>, TabChanged, TWindow>(addr_for_sub);
            bus.subscribe::<ProcessActor<P>, WslAgentRuntimeEvent, TWindow>(addr_for_wsl_sub);
        });

        let a = addr.clone();
        ui_port.on_select_process(move |pid, idx| {
            a.send(Select {
                pid: pid as u32,
                idx: idx as usize,
            })
        });

        addr.send(ScanTick);
        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(scan_interval_ms), move || {
            a.send(ScanTick)
        });

        Ok(())
    }
}

fn load_width_config(settings: &SettingsStore) -> ColumnWidthConfig {
    let mut cfg = ColumnWidthConfig::default();

    let default_width =
        ProcessSettings::get_or(settings, COLUMNS_DEFAULT_WIDTH_PX, 70u64).clamp(40, 1000) as u32;

    cfg.default_width_px = default_width;

    let cpu_w = ProcessSettings::get_or(settings, COLUMNS_CPU_WIDTH_PX, default_width as u64)
        .clamp(40, 1000) as u32;

    let mem_w =
        ProcessSettings::get_or(settings, COLUMNS_MEMORY_WIDTH_PX, 120u64).clamp(40, 1000) as u32;

    let mem_min = ProcessSettings::get_or(settings, COLUMNS_MEMORY_MIN_WIDTH_PX, mem_w as u64)
        .clamp(40, 1000) as u32;

    let disk_w = ProcessSettings::get_or(settings, COLUMNS_DISK_WIDTH_PX, default_width as u64)
        .clamp(40, 1000) as u32;

    let net_w = ProcessSettings::get_or(settings, COLUMNS_NET_WIDTH_PX, default_width as u64)
        .clamp(40, 1000) as u32;

    cfg.widths_px.insert("cpu", cpu_w);
    cfg.widths_px.insert("memory", mem_w);
    cfg.widths_px.insert("disk_read", disk_w);
    cfg.widths_px.insert("net", net_w);
    cfg.min_widths_px.insert("memory", mem_min);

    cfg
}
