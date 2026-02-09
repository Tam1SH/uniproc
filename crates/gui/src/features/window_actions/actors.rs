use crate::core::actor::traits::Message;
use crate::core::actor::traits::{Context, Handler};
use crate::{messages, AppWindow};
use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;

messages! {
    Drag,
    Close,
    Minimize,
    Maximize,
    Resize(i32)
}

pub struct WindowActor;

impl Handler<Drag, AppWindow> for WindowActor {
    fn handle(&mut self, _msg: Drag, ctx: &Context<Self, AppWindow>) {
        if let Some(ui) = ctx.ui_weak.upgrade() {
            ui.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    }
}

impl Handler<Close, AppWindow> for WindowActor {
    fn handle(&mut self, _msg: Close, ctx: &Context<Self, AppWindow>) {
        if let Some(ui) = ctx.ui_weak.upgrade() {
            let _ = ui.hide();
        }
    }
}

impl Handler<Minimize, AppWindow> for WindowActor {
    fn handle(&mut self, _msg: Minimize, ctx: &Context<Self, AppWindow>) {
        if let Some(ui) = ctx.ui_weak.upgrade() {
            ui.window().set_minimized(true);
        }
    }
}

impl Handler<Maximize, AppWindow> for WindowActor {
    fn handle(&mut self, _msg: Maximize, ctx: &Context<Self, AppWindow>) {
        if let Some(ui) = ctx.ui_weak.upgrade() {
            let w = ui.window();
            let is_max = w.is_maximized();
            w.set_maximized(!is_max);
        }
    }
}

impl Handler<Resize, AppWindow> for WindowActor {
    fn handle(&mut self, msg: Resize, ctx: &Context<Self, AppWindow>) {
        if let Some(ui) = ctx.ui_weak.upgrade() {
            use winit::window::ResizeDirection;
            let dir = match msg.0 {
                0 => ResizeDirection::North,
                1 => ResizeDirection::South,
                2 => ResizeDirection::West,
                3 => ResizeDirection::East,
                4 => ResizeDirection::NorthWest,
                5 => ResizeDirection::NorthEast,
                6 => ResizeDirection::SouthWest,
                7 => ResizeDirection::SouthEast,
                _ => return,
            };
            ui.window().with_winit_window(|w| {
                let _ = w.drag_resize_window(dir);
            });
        }
    }
}
