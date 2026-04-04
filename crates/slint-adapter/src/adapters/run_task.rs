use crate::native_windows::{NativeWindowConfig, NativeWindowManager};
use crate::{AppWindow, RunTaskDialog, RunTaskProxy, Theme};
use app_contracts::features::cosmetics::AccentColor;
use app_contracts::features::run_task::{RunTaskPort, RunTaskRequest};
use macros::ui_adapter;
use slint::ComponentHandle;
use std::rc::Rc;

#[derive(Clone)]
pub struct RunTaskAdapter {
    ui: slint::Weak<AppWindow>,
    dialog: NativeWindowManager<RunTaskDialog>,
}

impl RunTaskAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> anyhow::Result<Self> {
        let dialog = Rc::new(RunTaskDialog::new()?);
        Ok(Self {
            ui,
            dialog: NativeWindowManager::with_config(
                dialog,
                NativeWindowConfig::win11_dialog(),
            ),
        })
    }
}

#[ui_adapter]
impl RunTaskPort for RunTaskAdapter {
    #[ui_action(scope = "ui.run_task.open")]
    fn on_open<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<RunTaskProxy>().on_open(handler);
    }

    #[ui_action(scope = "ui.run_task.drag")]
    fn on_drag<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.dialog.component().global::<RunTaskProxy>().on_drag(handler);
    }

    #[ui_action(scope = "ui.run_task.submit", target = "request")]
    fn on_run_task<F>(&self, handler: F)
    where
        F: Fn(RunTaskRequest) + 'static,
    {
        self.dialog
            .component()
            .global::<RunTaskProxy>()
            .on_run_task(move |env, cmd| {
                handler(RunTaskRequest {
                    env_id: env.to_string(),
                    command: cmd.to_string(),
                });
            });
    }

    fn show_dialog(&self) {
        let _ = self.dialog.show();
    }

    fn hide_dialog(&self) {
        let _ = self.dialog.hide();
    }

    fn drag_dialog_window(&self) {
        self.dialog.drag_window();
    }

    fn apply_dialog_effects(&self) {
        self.dialog.apply_effects();
    }

    fn set_dialog_accent(&self, accent: AccentColor) {
        self.dialog
            .component()
            .global::<Theme>()
            .set_accent(slint::Color::from_argb_u8(
                accent.a, accent.r, accent.g, accent.b,
            ));
    }
}
