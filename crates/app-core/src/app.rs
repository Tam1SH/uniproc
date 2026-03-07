use crate::SharedState;
use crate::reactor::Reactor;
use slint::ComponentHandle;

pub trait FromUiWeak<TWindow: ComponentHandle + 'static>: Sized {
    fn from_ui_weak(ui: slint::Weak<TWindow>) -> Self;
}

pub trait Feature<TWindow: ComponentHandle + 'static>: Sized {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()>;
}

pub struct App<TWindow: ComponentHandle + 'static> {
    ui: TWindow,
    reactor: Reactor,
    shared: SharedState,
}

impl<TWindow: ComponentHandle + 'static> App<TWindow> {
    pub fn new(ui: TWindow) -> Self {
        Self {
            ui,
            reactor: Reactor::new(),
            shared: SharedState::new(),
        }
    }

    pub fn feature(self, feature: impl Feature<TWindow>) -> anyhow::Result<Self> {
        let mut this = self;
        feature.install(&mut this.reactor, &this.ui, &this.shared)?;
        Ok(this)
    }

    pub fn run(self) -> anyhow::Result<()> {
        self.ui.run()?;
        Ok(())
    }
}
