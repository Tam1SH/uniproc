use crate::settings::reactive::ReactiveSetting;
use std::time::Duration;

pub struct Reactor {
    _anchors: Vec<slint::Timer>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            _anchors: Vec::new(),
        }
    }

    pub fn add_loop(&mut self, interval: Duration, f: impl FnMut() + 'static) {
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, interval, f);
        self._anchors.push(timer);
    }

    pub fn add_dynamic_loop(
        &mut self,
        interval_ms_setting: &ReactiveSetting<u64>,
        f: impl FnMut() + 'static,
    ) {
        fn schedule_next(interval_ms_setting: ReactiveSetting<u64>, mut f: impl FnMut() + 'static) {
            let delay = Duration::from_millis(interval_ms_setting.get());

            slint::Timer::single_shot(delay, move || {
                f();
                schedule_next(interval_ms_setting, f);
            });
        }

        schedule_next(interval_ms_setting.clone(), f);
    }
}
