pub mod cosmetics;
pub mod envs;
pub mod processes;
pub mod window_actions;

pub mod context_menu;
pub mod navigation;

pub mod run_task;

use crate::core::reactor::Reactor;
use crate::AppWindow;

pub trait Feature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()>;
}
