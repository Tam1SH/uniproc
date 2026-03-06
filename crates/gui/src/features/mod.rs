pub mod cosmetics;
pub mod envs;
pub mod processes;
pub mod window_actions;

pub mod context_menu;
pub mod navigation;

pub mod l10n;
pub mod run_task;
pub mod settings;

use crate::AppWindow;
use crate::core::reactor::Reactor;
use app_core::SharedState;

pub trait Feature {
    fn install(
        self,
        reactor: &mut Reactor,
        ui: &AppWindow,
        shared: &SharedState,
    ) -> anyhow::Result<()>;
}
