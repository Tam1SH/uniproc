// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// extern crate core;

use crate::core::app::App;
use crate::features::context_menu::ContextMenuFeature;
use crate::features::cosmetics::CosmeticsFeature;
use crate::features::envs::EnvironmentsFeature;
use crate::features::navigation::NavigationFeature;
use crate::features::processes::ProcessFeature;
use crate::features::window_actions::WindowActionsFeature;

pub use app_core::messages;

mod core;
mod features;

mod scanner;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let rt = tokio::runtime::Runtime::new()?;

    let _guard = rt.enter();

    App::new()?
        .feature(CosmeticsFeature)?
        .feature(EnvironmentsFeature)?
        .feature(ProcessFeature { show_icons: true })?
        .feature(WindowActionsFeature)?
        .feature(ContextMenuFeature)?
        .feature(NavigationFeature)?
        .run()
}
