use app_core::app::Window;
use app_contracts::features::cosmetics::{AccentColor, CosmeticsPort};
use app_core::SharedState;
use app_core::app::Feature;
use app_core::reactor::Reactor;
use slint::ComponentHandle;

#[derive(Clone, Copy, Debug)]
pub struct AccentState(pub AccentColor);

pub struct CosmeticsFeature<F> {
    make_port: F,
}

impl<F> CosmeticsFeature<F> {
    pub fn new(make_port: F) -> Self {
        Self { make_port }
    }
}

impl<TWindow, F, P> Feature<TWindow> for CosmeticsFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static,
    P: CosmeticsPort,
{
    fn install(
        self,
        _reactor: &mut Reactor,
        ui: &TWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()> {
        let port = (self.make_port)(ui);
        if let Some(accent) = port.get_system_accent_color() {
            port.set_main_window_accent(accent);
            shared.insert(AccentState(accent));
        }
        port.apply_main_window_effects();
        Ok(())
    }
}

pub fn accent_from(shared: &SharedState) -> Option<AccentColor> {
    shared.get::<AccentState>().map(|x| x.0)
}
