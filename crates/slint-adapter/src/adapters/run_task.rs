use crate::{AppWindow, RunTaskDialog, RunTaskProxy, Theme};
use app_contracts::features::cosmetics::AccentColor;
use app_contracts::features::run_task::{RunTaskPort, RunTaskRequest};
use i_slint_backend_winit::WinitWindowAccessor;
use macros::ui_adapter;
use slint::ComponentHandle;
use std::rc::Rc;

#[derive(Clone)]
pub struct RunTaskAdapter {
    ui: slint::Weak<AppWindow>,
    dialog: Rc<RunTaskDialog>,
}

impl RunTaskAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> anyhow::Result<Self> {
        let dialog = RunTaskDialog::new()?;
        Ok(Self {
            ui,
            dialog: Rc::new(dialog),
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
        self.dialog.global::<RunTaskProxy>().on_drag(handler);
    }

    #[ui_action(scope = "ui.run_task.submit", target = "request")]
    fn on_run_task<F>(&self, handler: F)
    where
        F: Fn(RunTaskRequest) + 'static,
    {
        self.dialog
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
        self.dialog.window().with_winit_window(|w| {
            let _ = w.drag_window();
        });
    }

    fn apply_dialog_effects(&self) {
        #[cfg(target_os = "windows")]
        self.dialog.window().with_winit_window(|w| {
            let _ = window_vibrancy::apply_mica(w, Some(true));
        });
    }

    fn set_dialog_accent(&self, accent: AccentColor) {
        self.dialog
            .global::<Theme>()
            .set_accent(slint::Color::from_argb_u8(
                accent.a, accent.r, accent.g, accent.b,
            ));
    }
}
