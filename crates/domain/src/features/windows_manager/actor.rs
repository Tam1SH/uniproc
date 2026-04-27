use app_contracts::features::windows_manager::OpenedWindow;
use app_core::actor::event_bus::EventBus;
use app_core::actor::{Context, ManagedActor};
use framework::native_windows::slint_factory::{OpenWindow, WindowClosed, WindowRegistry};
use macros::{actor_manifest, handler};
use std::sync::Arc;

#[actor_manifest]
impl<R: WindowRegistry + 'static> ManagedActor for WindowManagerActor<R> {
    type Bus = bus!(
        @OpenWindow,
        @WindowClosed
    );
    type Handlers = handlers!(
        @OpenWindow,
        @WindowClosed
    );
}

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
