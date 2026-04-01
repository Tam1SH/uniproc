use app_core::app::Feature;
use app_core::app::Window;
use app_core::reactor::Reactor;
use app_core::SharedState;
use context::l10n::{L10nManager, L10nPort};

pub struct L10nFeature<F> {
    make_port: F,
}

impl<F> L10nFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for L10nFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: L10nPort,
{
    fn install(self, _: &mut Reactor, ui: &TWindow, _: &SharedState) -> anyhow::Result<()> {
        let port = (self.make_port)(ui);
        L10nManager::apply_to_port(&port);
        Ok(())
    }
}
