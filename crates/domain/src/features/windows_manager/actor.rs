use app_contracts::features::windows_manager::OpenedWindow;
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::{Context, Handler};
use context::native_windows::slint_factory::{OpenWindow, WindowClosed, WindowRegistry};
use macros::handler;
use std::sync::Arc;

pub struct WindowManagerActor<R> {
    registry: Arc<R>,
}

impl<R: WindowRegistry + 'static> WindowManagerActor<R> {
    pub fn new(registry: Arc<R>) -> Self {
        Self { registry }
    }
}

#[handler]
fn open_window<R: WindowRegistry + 'static>(
    this: &mut WindowManagerActor<R>,
    msg: OpenWindow,
    ctx: &Context<WindowManagerActor<R>>,
) {
    if this
        .registry
        .build_window(&ctx.addr().get_token(), &msg.template, &msg.key)
        .is_some()
    {
        EventBus::publish(OpenedWindow {
            key: msg.key,
            data: msg.data,
        });
    }
}

#[handler]
fn on_window_closed<R: WindowRegistry + 'static>(_: &mut WindowManagerActor<R>, _: WindowClosed) {}
