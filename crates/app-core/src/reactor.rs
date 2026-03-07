pub struct Reactor {
    _anchors: Vec<slint::Timer>,
}

impl Reactor {
    pub fn new() -> Self {
        Self {
            _anchors: Vec::new(),
        }
    }

    pub fn add_loop(&mut self, interval: std::time::Duration, f: impl FnMut() + 'static) {
        let timer = slint::Timer::default();
        timer.start(slint::TimerMode::Repeated, interval, f);
        self._anchors.push(timer);
    }
}
