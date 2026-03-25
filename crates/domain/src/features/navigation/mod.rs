use app_contracts::features::navigation::{
    NavigationUiBindings, NavigationUiPort, PageEntryDto, TabChanged, tab_name_by_index,
};
use app_core::SharedState;
use app_core::actor::event_bus::EVENT_BUS;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use app_core::settings::{FeatureSettings, SettingsScope, SettingsStore, settings_from};
use slint::ComponentHandle;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

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

pub struct NavigationFeature<F> {
    make_ui_port: F,
}

impl<F> NavigationFeature<F> {
    pub fn new(make_ui_port: F) -> Self {
        Self { make_ui_port }
    }
}

fn animate_switch_progress<P>(
    ui_port: P,
    token: Rc<Cell<u64>>,
    active_token: u64,
    started_at: Instant,
    duration: Duration,
) where
    P: NavigationUiPort + Clone + 'static,
{
    slint::Timer::single_shot(Duration::from_millis(16), move || {
        if token.get() != active_token {
            return;
        }

        let elapsed = started_at.elapsed().as_secs_f32();
        let total = duration.as_secs_f32().max(0.001);
        let t = (elapsed / total).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        ui_port.set_switch_progress(eased);

        if t < 1.0 {
            animate_switch_progress(
                ui_port.clone(),
                token.clone(),
                active_token,
                started_at,
                duration,
            );
        } else {
            ui_port.set_switch_progress(1.0);
        }
    });
}

impl<TWindow, F, P> Feature<TWindow> for NavigationFeature<F>
where
    TWindow: ComponentHandle + 'static,
    F: Fn(&TWindow) -> P + 'static,
    P: NavigationUiPort + NavigationUiBindings + Clone + 'static,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let settings = settings_from(shared);
        NavigationSettings::ensure_defaults(&settings)?;

        let hide_delay_ms = NavigationSettings::setting_or(&settings, SWITCH_HIDE_DELAY_MS, 60u64)?;
        let show_delay_ms = NavigationSettings::setting_or(&settings, SWITCH_SHOW_DELAY_MS, 20u64)?;

        let ui_port = (self.make_ui_port)(ui);
        ui_port.set_pages(vec![
            PageEntryDto {
                id: 0,
                text: "Processes".into(),
                icon_key: "apps-list".into(),
            },
            PageEntryDto {
                id: 1,
                text: "Performance".into(),
                icon_key: "data-area".into(),
            },
            PageEntryDto {
                id: 2,
                text: "Disk".into(),
                icon_key: "database".into(),
            },
            PageEntryDto {
                id: 3,
                text: "Statistics".into(),
                icon_key: "statistics".into(),
            },
            PageEntryDto {
                id: 4,
                text: "Startup apps".into(),
                icon_key: "dashed-settings".into(),
            },
            PageEntryDto {
                id: 5,
                text: "Users".into(),
                icon_key: "people".into(),
            },
            PageEntryDto {
                id: 6,
                text: "Services".into(),
                icon_key: "puzzle".into(),
            },
        ]);

        let ui_for_switch = ui_port.clone();
        let switch_anim_token = Rc::new(Cell::new(0u64));
        let switch_anim_duration = Duration::from_millis(320);
        ui_port.on_request_tab_switch(move |new_index| {
            let event = TabChanged {
                name: tab_name_by_index(new_index).to_string(),
            };
            EVENT_BUS.with(|bus| bus.publish(event));

            let current_index = ui_for_switch.get_active_tab_index();
            if current_index == new_index {
                return;
            }

            ui_for_switch.set_switch_transition(current_index, new_index, 0.0);
            let next_token = switch_anim_token.get().wrapping_add(1);
            switch_anim_token.set(next_token);
            animate_switch_progress(
                ui_for_switch.clone(),
                switch_anim_token.clone(),
                next_token,
                Instant::now(),
                switch_anim_duration,
            );

            let hide_delay_ms = hide_delay_ms.clone();
            let show_delay_ms = show_delay_ms.clone();
            ui_for_switch.set_content_visible(false);
            let ui_after_hide = ui_for_switch.clone();
            slint::Timer::single_shot(Duration::from_millis(hide_delay_ms.get().max(1)), move || {
                ui_after_hide.set_active_tab_index(new_index);
                let ui_after_show = ui_after_hide.clone();
                slint::Timer::single_shot(Duration::from_millis(show_delay_ms.get().max(1)), move || {
                    ui_after_show.set_content_visible(true);
                });
            });
        });

        Ok(())
    }
}
