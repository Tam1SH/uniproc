use crate::signal::Signal;
use std::sync::Arc;
use std::time::Duration;

pub struct Reactor {
    _anchors: Vec<slint::Timer>,
}

pub struct DynamicLoopControl {
    active: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for DynamicLoopControl {
    fn drop(&mut self) {
        self.active
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            _anchors: Vec::new(),
        }
    }

    pub fn add_dynamic_loop(
        &mut self,
        interval_ms_setting: Arc<Signal<u64>>,
        f: impl FnMut() + 'static,
    ) -> DynamicLoopControl {
        use std::sync::atomic::{AtomicBool, Ordering};

        let active = Arc::new(AtomicBool::new(true));
        let active_clone = active.clone();

        fn schedule_next(
            interval_ms_setting: Arc<Signal<u64>>,
            active: Arc<AtomicBool>,
            mut f: impl FnMut() + 'static,
        ) {
            let delay = Duration::from_millis(interval_ms_setting.get());

            slint::Timer::single_shot(delay, move || {
                if !active.load(Ordering::Relaxed) {
                    return;
                }
                f();
                schedule_next(interval_ms_setting, active, f);
            });
        }

        schedule_next(interval_ms_setting, active_clone.clone(), f);

        DynamicLoopControl { active }
    }
}
