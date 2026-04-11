mod apply;

use app_contracts::features::l10n::L10nPort;
use app_core::app::{Feature, Window};
use app_core::reactor::Reactor;
use app_core::SharedState;

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
        apply::apply(&port);
        Ok(())
    }
}
