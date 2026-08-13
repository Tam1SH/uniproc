pub mod actor;
pub mod backend;
pub mod connection;
pub mod decode;
pub mod providers;
pub mod rpc;
pub mod settings;

use app_contracts2::features::agents::ScanTick;
use guinea::feature::{AppFeature, AppFeatureInitContext, ContextStoreExt};
use guinea_core::actor::event_bus::GlobalEventBus;
use guinea_core::signal::Signal;
use guinea_macros::app_feature;
use settings::AgentSettings;
use tracing::info;

const MIN_SCAN_INTERVAL_MS: u64 = 100;

#[app_feature]
pub fn agents_feature(ctx: &mut AppFeatureInitContext) -> anyhow::Result<()> {
    info!("Agents feature installed");

    let store = ctx.store();
    let settings = AgentSettings::new_with(&store)?;
    let interval = Signal::new(settings.scan_interval_ms().get().max(MIN_SCAN_INTERVAL_MS));
    ctx.reactor.add_heartbeat(interval, || {
        GlobalEventBus::publish(ScanTick);
    });

    cfg_if::cfg_if! {
        if #[cfg(target_os = "windows")] {
            providers::wsl::wsl_agent_feature(ctx)?;
            providers::windows::windows_agent_feature(ctx)?;
        } else {
            providers::linux::linux_agent_feature(ctx)?;
        }
    }

    Ok(())
}
