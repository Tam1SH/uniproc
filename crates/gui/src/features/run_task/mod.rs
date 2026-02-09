use crate::core::actor::addr::Addr;
use crate::core::reactor::Reactor;
use crate::features::cosmetics::utils::{apply_native_win11_style, WindowTexture};
use crate::features::cosmetics::CosmeticsFeature;
use crate::features::run_task::actors::{Drag, Execute, RunTaskActor, Show};
use crate::features::Feature;
use crate::{AppWindow, RunTaskDialog, RunTaskProxy, Theme};
use slint::ComponentHandle;

mod actors;

pub struct RunTaskFeature;

impl Feature for RunTaskFeature {
    fn install(self, _: &mut Reactor, main_ui: &AppWindow) -> anyhow::Result<()> {
        let dialog = RunTaskDialog::new()?;

        let dialog_weak = dialog.as_weak();
        let state = RunTaskActor { window: dialog };
        let addr = Addr::new(state, main_ui.as_weak());

        main_ui.global::<RunTaskProxy>().on_open(addr.handler(Show));

        let _ = slint::spawn_local(async move {
            let Some(ui) = dialog_weak.upgrade() else {
                return;
            };

            let proxy = ui.global::<RunTaskProxy>();

            proxy.on_drag(addr.handler(Drag));

            let a = addr.clone();

            proxy.on_run_task(move |env, cmd| {
                a.send(Execute {
                    env_id: env,
                    command: cmd,
                });
            });

            apply_native_win11_style(ui.window(), WindowTexture::Mica).await;

            if let Some(accent) = CosmeticsFeature::get_system_accent_color() {
                ui.global::<Theme>().set_accent_color(accent);
            }
        });

        Ok(())
    }
}
