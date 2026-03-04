// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// extern crate core;

#[macro_use]
extern crate rust_i18n;

use crate::core::app::App;
use crate::features::context_menu::ContextMenuFeature;
use crate::features::cosmetics::CosmeticsFeature;
use crate::features::envs::EnvironmentsFeature;
use crate::features::l10n::L10nFeature;
use crate::features::navigation::NavigationFeature;
use crate::features::processes::ProcessFeature;
use crate::features::window_actions::WindowActionsFeature;
pub use app_core::messages;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

i18n!("locales", fallback = "en");

mod core;
mod features;
mod shared;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    let targets = Targets::new()
        .with_default(Level::DEBUG)
        .with_target("ogurpchik", Level::WARN);

    tracing_subscriber::registry()
        .with(targets)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let rt = tokio::runtime::Runtime::new()?;

    let _guard = rt.enter();

    App::new()?
        .feature(CosmeticsFeature)?
        .feature(EnvironmentsFeature)?
        .feature(ProcessFeature { show_icons: true })?
        .feature(WindowActionsFeature)?
        .feature(ContextMenuFeature)?
        .feature(NavigationFeature)?
        .feature(L10nFeature)?
        .run()
}
