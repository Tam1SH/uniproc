use app_contracts::features::windows_manager::OpenedWindow;
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;
use context::native_windows::slint_factory::{OpenWindow, WindowClosed, WindowRegistry};
use std::sync::Arc;

pub struct WindowManagerActor<R> {
    registry: Arc<R>,
}

impl<R: WindowRegistry + 'static> WindowManagerActor<R> {
    pub fn new(registry: Arc<R>) -> Self {
        Self { registry }
    }
}

impl<R: WindowRegistry + 'static, TWindow: Window> Handler<OpenWindow, TWindow>
    for WindowManagerActor<R>
{
    fn handle(&mut self, msg: OpenWindow, ctx: &Context<Self, TWindow>) {
        if self
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
}

impl<R: WindowRegistry + 'static, TWindow: Window> Handler<WindowClosed, TWindow>
    for WindowManagerActor<R>
{
    fn handle(&mut self, msg: WindowClosed, _ctx: &Context<Self, TWindow>) {}
}
