use std::cell::RefCell;
use std::rc::Rc;
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

    pub fn add_dynamic_loop<I>(&mut self, interval: I, f: impl FnMut() + 'static)
    where
        I: Fn() -> Duration + 'static,
    {
        fn schedule_next(
            interval: Rc<dyn Fn() -> Duration>,
            action: Rc<RefCell<dyn FnMut()>>,
        ) {
            let delay = (interval)().max(Duration::from_millis(1));
            let interval_next = Rc::clone(&interval);
            let action_next = Rc::clone(&action);

            slint::Timer::single_shot(delay, move || {
                (action_next.borrow_mut())();
                schedule_next(interval_next, action_next);
            });
        }

        let interval: Rc<dyn Fn() -> Duration> = Rc::new(interval);
        let action: Rc<RefCell<dyn FnMut()>> = Rc::new(RefCell::new(f));
        schedule_next(interval, action);
    }
}
