mod apply;

use app_contracts::features::l10n::L10nPort;
use framework::app::Window;
use framework::feature::{WindowFeature, WindowFeatureInitContext};
use macros::window_feature;

#[window_feature]
pub struct L10nFeature;

#[window_feature]
impl<TWindow, F, P> WindowFeature<TWindow> for L10nFeature<F>
where
    TWindow: Window,
    F: Fn(&TWindow) -> P + 'static + Clone,
    P: L10nPort,
{
    fn install(&mut self, ctx: &mut WindowFeatureInitContext<TWindow>) -> anyhow::Result<()> {
        let port = (self.make_port)(ctx.ui);
        apply::apply(&port);
        Ok(())
    }
}
