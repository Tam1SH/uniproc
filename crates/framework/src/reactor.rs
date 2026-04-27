use app_core::signal::Signal;
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

#[cfg(test)]
mod tests {
    use super::*;
    use app_core::signal::Signal;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_reactor_dynamic_loop() {
        i_slint_backend_testing::init_no_event_loop();

        let mut reactor = Reactor::new();
        let interval = Arc::new(Signal::new(1000));
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        let control = reactor.add_dynamic_loop(interval.clone(), move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 0);

        i_slint_core::tests::slint_mock_elapsed_time(1001);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        interval.set(500);

        i_slint_core::tests::slint_mock_elapsed_time(1000);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        i_slint_core::tests::slint_mock_elapsed_time(250);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        i_slint_core::tests::slint_mock_elapsed_time(251);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        drop(control);

        i_slint_core::tests::slint_mock_elapsed_time(2000);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    #[test]
    fn test_reactor_drop_control_immediately() {
        i_slint_backend_testing::init_no_event_loop();
        let mut reactor = Reactor::new();
        let counter = Arc::new(AtomicUsize::new(0));

        {
            let _control = reactor.add_dynamic_loop(Arc::new(Signal::new(10)), {
                let c = counter.clone();
                move || {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            });
        }

        i_slint_core::tests::slint_mock_elapsed_time(100);
        slint::platform::update_timers_and_animations();
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
