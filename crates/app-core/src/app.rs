use crate::SharedState;
use crate::actor::UiThreadGuard;
use crate::reactor::Reactor;
use slint::ComponentHandle;

pub trait FromUiWeak<TWindow: ComponentHandle + 'static>: Sized {
    fn from_ui_weak(ui: slint::Weak<TWindow>) -> Self;
}

pub trait UiContext {
    fn new_token(&self) -> UiThreadGuard;
}

impl<TWindow: ComponentHandle + 'static> UiContext for TWindow {
    fn new_token(&self) -> UiThreadGuard {
        UiThreadGuard::new()
    }
}

pub trait Window: ComponentHandle + UiContext + 'static {}
impl<TWindow: ComponentHandle + UiContext + 'static> Window for TWindow {}

pub trait Feature<TWindow: Window>: Sized {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()>;
}

pub struct App<TWindow> {
    ui: TWindow,
    reactor: Reactor,
    shared: SharedState,
}

impl<TWindow: Window> App<TWindow> {
    pub fn new(ui: TWindow) -> Self {
        Self {
            ui,
            reactor: Reactor::new(),
            shared: SharedState::new(),
        }
    }

    pub fn feature<F: Feature<TWindow>>(mut self, feature: F) -> anyhow::Result<Self> {
        let full_name = std::any::type_name::<F>();

        let clean_name = full_name
            .split('<')
            .next()
            .unwrap_or(full_name)
            .split("::")
            .last()
            .unwrap_or("Unknown");

        let _span = tracing::info_span!("install", feature = %clean_name).entered();

        match feature.install(&mut self.reactor, &self.ui, &self.shared) {
            Ok(_) => {
                tracing::info!("Success");
                Ok(self)
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed");
                Err(e)
            }
        }
    }

    pub fn run(self) -> anyhow::Result<()> {
        self.ui.run()?;
        Ok(())
    }
}
