use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::shared::l10n::L10nManager;
use crate::AppWindow;
use slint::ComponentHandle;

pub struct L10nFeature;

impl Feature for L10nFeature {
    fn install(self, _: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        L10nManager::apply_to_global(&ui.global());
        Ok(())
    }
}
