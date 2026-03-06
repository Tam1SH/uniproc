use crate::core::actor::event_bus::EVENT_BUS;
use crate::core::actor::traits::Message;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::features::navigation::utils::get_tab_name_by_index;
use crate::features::settings::{SettingsStore, settings_from};
use crate::shared::icons::Icons;
use crate::shared::settings::{FeatureSettings, SettingsScope};
use crate::{AppWindow, Dashboard, messages};
use crate::{Navigation, PageEntry};
use app_core::SharedState;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::Duration;

pub mod utils;

const SWITCH_HIDE_DELAY_MS: &str = "switch_hide_delay_ms";
const SWITCH_SHOW_DELAY_MS: &str = "switch_show_delay_ms";

struct NavigationSettings;

impl SettingsScope for NavigationSettings {
    const PREFIX: &'static str = "navigation";
}

impl FeatureSettings for NavigationSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, SWITCH_HIDE_DELAY_MS, 60u64)?;
        Self::ensure_default(settings, SWITCH_SHOW_DELAY_MS, 20u64)?;
        Ok(())
    }
}

pub struct NavigationFeature;

impl Feature for NavigationFeature {
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);

        NavigationSettings::ensure_defaults(&settings)?;

        let hide_delay_ms =
            NavigationSettings::get_or(&settings, SWITCH_HIDE_DELAY_MS, 60u64).max(1);
        let show_delay_ms =
            NavigationSettings::get_or(&settings, SWITCH_SHOW_DELAY_MS, 20u64).max(1);
        let ui_handle = ui.as_weak();

        ui.global::<Dashboard>()
            .set_pages(ModelRc::new(VecModel::from(vec![
                PageEntry {
                    id: 0,
                    text: "Processes".into(),
                    icon: Icons::get("app"),
                },
                PageEntry {
                    id: 1,
                    text: "Performance".into(),
                    icon: Icons::get("data-area"),
                },
                PageEntry {
                    id: 2,
                    text: "Disk".into(),
                    icon: Icons::get("database"),
                },
                PageEntry {
                    id: 3,
                    text: "Statistics".into(),
                    icon: Icons::get("statistics"),
                },
                PageEntry {
                    id: 4,
                    text: "Startup apps".into(),
                    icon: Icons::get("dashed-settings"),
                },
                PageEntry {
                    id: 5,
                    text: "Users".into(),
                    icon: Icons::get("people"),
                },
                PageEntry {
                    id: 6,
                    text: "Services".into(),
                    icon: Icons::get("puzzle"),
                },
            ])));

        ui.global::<Navigation>()
            .on_request_tab_switch(move |new_index| {
                let ui = match ui_handle.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };

                let tab_name = get_tab_name_by_index(new_index);

                let event = TabChanged {
                    name: tab_name.to_string().into(),
                };
                EVENT_BUS.with(|bus| bus.publish(event));

                let nav = ui.global::<Navigation>();

                if nav.get_active_tab_index() == new_index {
                    return;
                }

                nav.set_content_visible(false);

                slint::Timer::single_shot(Duration::from_millis(hide_delay_ms), move || {
                    let nav = ui.global::<Navigation>();

                    nav.set_active_tab_index(new_index);

                    slint::Timer::single_shot(Duration::from_millis(show_delay_ms), move || {
                        ui.global::<Navigation>().set_content_visible(true);
                    });
                });
            });

        Ok(())
    }
}

messages! {
    TabChanged {
        name: SharedString
    }
}
