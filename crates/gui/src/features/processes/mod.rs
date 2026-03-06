use super::Feature;
use crate::core::reactor::Reactor;
use crate::{AppWindow, Navigation, ProcessGroup, ProcessesFeatureGlobal};

use crate::core::actor::addr::Addr;
use crate::core::actor::event_bus::EVENT_BUS;
use crate::features::envs::wsl::WslAgentRuntimeEvent;
use crate::features::navigation::utils::get_tab_name_by_index;
use crate::features::navigation::TabChanged;
use crate::features::processes::application::actors::*;
use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::scanner::wsl::{SharedWslClient, WslScanner};
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::processes::ui::slint_bridge::ColumnWidthConfig;
use crate::features::run_task::RunTaskFeature;
use crate::features::settings::{settings_from, SettingsStore};
use crate::shared::settings::{FeatureSettings, SettingsScope};
use app_core::SharedState;
use scanner::windows::WindowsScanner;
use slint::{ComponentHandle, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
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

pub struct ProcessFeature {
    pub show_icons: bool,
}

impl Feature for ProcessFeature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        RunTaskFeature.install(reactor, ui, shared)?;

        let settings = settings_from(shared);

        ProcessSettings::ensure_defaults(&settings)?;

        let scan_interval_ms = ProcessSettings::get_or(&settings, SCAN_INTERVAL_MS, 1500u64).max(1);

        let name_cache_ttl_secs =
            ProcessSettings::get_or(&settings, METADATA_NAME_CACHE_TTL_SECS, 300u64).max(1);

        let icon_cache_ttl_secs =
            ProcessSettings::get_or(&settings, METADATA_ICON_CACHE_TTL_SECS, 120u64).max(1);

        let width_config = load_width_config(&settings);

        let ui_model = Rc::new(VecModel::<ProcessGroup>::default());

        ui.global::<ProcessesFeatureGlobal>()
            .set_process_groups(ui_model.clone().into());

        let active_index = ui.global::<Navigation>().get_active_tab_index();
        let current_active_name = get_tab_name_by_index(active_index);
        let wsl_client: SharedWslClient = Arc::new(RwLock::new(None));

        let state = ProcessActor {
            flow: ProcessFlowState::new(self.show_icons),
            metadata: ProcessMetadataService::new(
                Duration::from_secs(name_cache_ttl_secs),
                Duration::from_secs(icon_cache_ttl_secs),
            ),
            ui_model: ui_model.clone(),
            scanners: Some(vec![
                Box::new(WindowsScanner::new()),
                Box::new(WslScanner::new(Arc::clone(&wsl_client))),
            ]),
            widths_by_schema: HashMap::new(),
            width_config,
            is_active: current_active_name == "Processes",
            wsl_client,
        };

        let addr = Addr::new(state, ui.as_weak());
        let bridge = ui.global::<ProcessesFeatureGlobal>();

        bridge.on_sort_by(addr.handler_with(Sort));
        bridge.on_toggle_expand_group(addr.handler_with(ToggleExpand));
        bridge.on_terminate(addr.handler(TerminateSelected));

        let addr_for_sub = addr.clone();
        let addr_for_wsl_sub = addr.clone();

        EVENT_BUS.with(|bus| {
            bus.subscribe::<ProcessActor, TabChanged, _>(addr_for_sub);
            bus.subscribe::<ProcessActor, WslAgentRuntimeEvent, _>(addr_for_wsl_sub);
        });

        let a = addr.clone();
        bridge.on_select_process(move |pid, idx| {
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
