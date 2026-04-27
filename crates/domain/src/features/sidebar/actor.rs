use crate::features::sidebar::settings::SidebarSettings;
use app_contracts::features::sidebar::{RequestTransition, UiSidebarPort};
use app_core::actor::ManagedActor;
use app_core::trace::{current_meta, install_current_meta};
use macros::{actor_manifest, handler};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[actor_manifest]
impl<P: UiSidebarPort + Clone> ManagedActor for SidebarActor<P> {
    type Bus = bus!(
        @RequestTransition
    );
    type Handlers = handlers!(
        @RequestTransition,
        SideBarWidthChanged(u64)
    );
}

pub struct SidebarActor<P: UiSidebarPort> {
    ui_port: P,
    settings: SidebarSettings,
    anim_token: Arc<AtomicU64>,
}

impl<P: UiSidebarPort + Clone> SidebarActor<P> {
    pub fn new(ui_port: P, settings: SidebarSettings) -> Self {
        Self {
            ui_port,
            settings,
            anim_token: Arc::new(AtomicU64::new(0)),
        }
    }

    fn run_animation_step(
        ui: P,
        token_ref: Arc<AtomicU64>,
        target_token: u64,
        start: Instant,
        duration: Duration,
    ) {
        let meta = current_meta();
        slint::Timer::single_shot(Duration::from_millis(16), move || {
            let _meta_guard = meta.clone().map(install_current_meta);
            if token_ref.load(Ordering::SeqCst) != target_token {
                return;
            }

            let elapsed = start.elapsed().as_secs_f32();
            let t = (elapsed / duration.as_secs_f32().max(0.001)).clamp(0.0, 1.0);

            let eased = if t < 0.5 {
                8.0 * t * t * t * t
            } else {
                1.0 - f32::powi(-2.0 * t + 2.0, 4) / 2.0
            };

            ui.set_switch_progress(eased);

            if t < 1.0 {
                Self::run_animation_step(ui, token_ref, target_token, start, duration);
            } else {
                ui.set_switch_progress(1.0);
            }
        });
    }
}

#[handler]
fn handle_transition<P: UiSidebarPort + Clone>(this: &mut SidebarActor<P>, msg: RequestTransition) {
    let ui = this.ui_port.clone();

    ui.set_switch_transition(msg.from_index, msg.to_index, 0.0);
    ui.set_content_visible(false);

    let next_token = this.anim_token.fetch_add(1, Ordering::SeqCst) + 1;
    let duration = Duration::from_millis(600);

    SidebarActor::<P>::run_animation_step(
        ui.clone(),
        this.anim_token.clone(),
        next_token,
        Instant::now(),
        duration,
    );

    let h_delay = Duration::from_millis(this.settings.switch_hide_delay_ms().get());
    let s_delay = Duration::from_millis(this.settings.switch_show_delay_ms().get());
    let meta = current_meta();

    slint::Timer::single_shot(h_delay, move || {
        let _meta_guard = meta.clone().map(install_current_meta);
        let ui_inner = ui.clone();
        slint::Timer::single_shot(s_delay, move || {
            ui_inner.set_content_visible(true);
        });
    });
}

#[handler]
fn handle_width_change<P: UiSidebarPort>(this: &mut SidebarActor<P>, msg: SideBarWidthChanged) {
    this.settings.set_width(msg.0).ok();
}
