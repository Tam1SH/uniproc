use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::context_menu::actors::*;
use crate::features::context_menu::utils::configure_window_styles;
use crate::features::cosmetics::utils::{apply_native_win11_style, WindowTexture};
use crate::features::cosmetics::CosmeticsFeature;
use crate::features::settings::{settings_from, SettingsStore};
use crate::features::Feature;
use crate::shared::settings::{FeatureSettings, SettingsScope};
use crate::{AppWindow, Theme};
use crate::{ContextMenuProxy, ProcessContextMenu};
use app_core::SharedState;
use slint::ComponentHandle;
use std::cell::RefCell;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;

mod actors;
pub mod utils;

const REVEAL_DELAY_MS: &str = "reveal_delay_ms";

struct ContextMenuSettings;

impl SettingsScope for ContextMenuSettings {
    const PREFIX: &'static str = "context_menu";
}

impl FeatureSettings for ContextMenuSettings {
    fn ensure_defaults(settings: &SettingsStore) -> anyhow::Result<()> {
        Self::ensure_default(settings, REVEAL_DELAY_MS, 20u64)
    }
}

thread_local! {
    static HOOK_HANDLE: RefCell<Option<HWINEVENTHOOK>> = RefCell::new(None);

    static ACTOR_ADDR: RefCell<Option<Addr<ContextMenuActor, AppWindow>>> = RefCell::new(None);
}

pub struct ContextMenuFeature;

impl Feature for ContextMenuFeature {
    fn install(self, _: &mut Reactor, ui: &AppWindow, shared: &SharedState) -> anyhow::Result<()> {
        let settings = settings_from(shared);

        ContextMenuSettings::ensure_defaults(&settings)?;

        let reveal_delay_ms = ContextMenuSettings::get_or(&settings, REVEAL_DELAY_MS, 20u64);

        let menu = ProcessContextMenu::new()?;

        configure_window_styles(&menu, 0);

        let menu_weak = menu.as_weak();

        let state = ContextMenuActor {
            menu,
            main_hwnd: 0,
            menu_hwnd: 0,
            reveal_delay_ms,
        };

        let addr = Addr::new(state, ui.as_weak());

        ACTOR_ADDR.with(|store| *store.borrow_mut() = Some(addr.clone()));

        let a = addr.clone();
        ui.global::<ContextMenuProxy>()
            .on_show_context_menu(move |x, y| {
                a.send(Show { x, y });
            });

        let a = addr.clone();
        ui.global::<ContextMenuProxy>().on_close_menu(move || {
            a.send(Hide);
        });

        slint::spawn_local(async move {
            if let Some(m) = menu_weak.upgrade() {
                m.on_action(addr.handler_with(HandleAction));

                apply_native_win11_style(m.window(), WindowTexture::None).await;
                if let Some(accent) = CosmeticsFeature::get_system_accent_color() {
                    m.global::<Theme>().set_accent(accent);
                }
            }
        })?;

        Ok(())
    }
}
