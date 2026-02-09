use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::AppWindow;
use slint::ComponentHandle;

pub struct App {
    ui: AppWindow,
    reactor: Reactor,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            ui: AppWindow::new()?,
            reactor: Reactor::new(),
        })
    }

    pub fn feature(mut self, feature: impl Feature) -> anyhow::Result<Self> {
        feature.install(&mut self.reactor, &self.ui)?;
        Ok(self)
    }

    pub fn run(self) -> anyhow::Result<()> {
        self.ui.run()?;
        Ok(())
    }
}
