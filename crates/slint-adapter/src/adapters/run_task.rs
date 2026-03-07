use crate::{AppWindow, RunTaskDialog, RunTaskProxy, Theme};
use app_contracts::features::cosmetics::AccentColor;
use app_contracts::features::run_task::{RunTaskPort, RunTaskRequest};
use i_slint_backend_winit::WinitWindowAccessor;
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

    fn with_ui<F>(&self, f: F)
    where
        F: FnOnce(&AppWindow),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui);
        }
    }

    fn with_dialog<F>(&self, f: F)
    where
        F: FnOnce(&RunTaskDialog),
    {
        f(self.dialog.as_ref());
    }
}

impl RunTaskPort for RunTaskAdapter {
    fn on_open<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(|ui| ui.global::<RunTaskProxy>().on_open(handler));
    }

    fn on_drag<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_dialog(|dialog| {
            dialog.global::<RunTaskProxy>().on_drag(handler);
        });
    }

    fn on_run_task<F>(&self, handler: F)
    where
        F: Fn(RunTaskRequest) + 'static,
    {
        self.with_dialog(|dialog| {
            dialog
                .global::<RunTaskProxy>()
                .on_run_task(move |env, cmd| {
                    handler(RunTaskRequest {
                        env_id: env.to_string(),
                        command: cmd.to_string(),
                    });
                });
        });
    }

    fn show_dialog(&self) {
        self.with_dialog(|dialog| {
            let _ = dialog.show();
        });
    }

    fn hide_dialog(&self) {
        self.with_dialog(|dialog| {
            let _ = dialog.hide();
        });
    }

    fn drag_dialog_window(&self) {
        self.with_dialog(|dialog| {
            dialog.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        });
    }

    fn apply_dialog_effects(&self) {
        #[cfg(target_os = "windows")]
        self.with_dialog(|dialog| {
            dialog.window().with_winit_window(|w| {
                let _ = window_vibrancy::apply_mica(w, Some(true));
            });
        });
    }

    fn set_dialog_accent(&self, accent: AccentColor) {
        self.with_dialog(|dialog| {
            dialog
                .global::<Theme>()
                .set_accent(slint::Color::from_argb_u8(
                    accent.a, accent.r, accent.g, accent.b,
                ));
        });
    }
}
