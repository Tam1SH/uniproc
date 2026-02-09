use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::context_menu::actors::*;
use crate::features::context_menu::utils::configure_window_styles;
use crate::features::cosmetics::utils::{apply_native_win11_style, WindowTexture};
use crate::features::cosmetics::CosmeticsFeature;
use crate::features::Feature;
use crate::AppWindow;
use crate::{ContextMenuProxy, ProcessContextMenu};
use i_slint_backend_winit::WinitWindowAccessor;
use raw_window_handle::HasWindowHandle;
use slint::ComponentHandle;
use std::cell::RefCell;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;

mod actors;
pub mod utils;

thread_local! {
    static HOOK_HANDLE: RefCell<Option<HWINEVENTHOOK>> = RefCell::new(None);

    static ACTOR_ADDR: RefCell<Option<Addr<ContextMenuActor, AppWindow>>> = RefCell::new(None);
}

pub struct ContextMenuFeature;

impl Feature for ContextMenuFeature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let menu = ProcessContextMenu::new()?;

        configure_window_styles(&menu, 0);

        let menu_weak = menu.as_weak();

        let state = ContextMenuActor {
            menu,
            main_hwnd: 0,
            menu_hwnd: 0,
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
                    m.global::<crate::Theme>().set_accent_color(accent);
                }
            }
        })?;

        Ok(())
    }
}
