use crate::AppWindow;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use app_core::SharedState;
use slint::ComponentHandle;

pub struct App {
    ui: AppWindow,
    reactor: Reactor,
    shared: SharedState,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            ui: AppWindow::new()?,
            reactor: Reactor::new(),
            shared: SharedState::new(),
        })
    }

    pub fn feature(mut self, feature: impl Feature) -> anyhow::Result<Self> {
        feature.install(&mut self.reactor, &self.ui, &self.shared)?;
        Ok(self)
    }

    pub fn run(self) -> anyhow::Result<()> {
        self.ui.run()?;
        Ok(())
    }
}
