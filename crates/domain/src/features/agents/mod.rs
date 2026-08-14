pub mod actor;
pub mod backend;
pub mod connection;
pub mod decode;
pub mod providers;
pub mod rpc;
pub mod settings;

use app_contracts::features::agents::ScanTick;
use guinea::app::{AppFeature, FeatureBuilder};
use guinea::feature::FeatureContext;
use guinea_core::actor::event_bus::GlobalEventBus;
use settings::AgentSettings;
use tracing::info;

const MIN_SCAN_INTERVAL_MS: u64 = 100;

pub struct AgentsFeature;

impl AppFeature for AgentsFeature {
    fn install(self, app: &mut FeatureBuilder) -> anyhow::Result<()> {
        info!("Agents feature installed");

        let settings = AgentSettings::new()?;
        let interval = settings.scan_interval_ms().get().max(MIN_SCAN_INTERVAL_MS);
        let heartbeat = app.reactor().add_heartbeat(move || interval, || {
            GlobalEventBus::publish(ScanTick);
        });
        app.tracker().track_loop(heartbeat);

        cfg_if::cfg_if! {
            if #[cfg(target_os = "windows")] {
                providers::wsl::wsl_agent_feature(app)?;
                providers::windows::windows_agent_feature(app)?;
            } else {
                providers::linux::linux_agent_feature(app)?;
            }
        }

        Ok(())
    }
}
