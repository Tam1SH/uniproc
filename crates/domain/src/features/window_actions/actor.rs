use app_contracts::features::window_actions::{
    ResizeEdge, UiWindowActionsBindings, UiWindowActionsPort, UiWindowActionsPortMsg,
    WindowActionsBinder, WindowActionsPartialBinder, WindowBreakpoint, WindowConfigChanged,
};
use forsl_core::actor::{Context, ManagedActor};
use forsl_macros::handler;
use macros::actor_manifest;

#[actor_manifest(binder = WindowActionsBinder)]
impl<P: UiWindowActionsPort> ManagedActor for WindowActor<P> {
    type Bus = bus!();
    type Handlers = handlers!(
        bind {
            Drag,
            Close,
            Minimize,
            Maximize,
            StartResize(ResizeEdge),
            ConfigChanged(WindowBreakpoint, u64)
        },
    );
    type Signals = bus!(WindowConfigChanged);
}

pub struct WindowActor<P> {
    pub port: P,
}

#[handler]
fn drag_window<P: UiWindowActionsPort>(this: &WindowActor<P>, _: Drag) {
    this.port.send(UiWindowActionsPortMsg::Drag);
}

#[handler]
fn close_window<P: UiWindowActionsPort>(this: &WindowActor<P>, _: Close) {
    this.port.send(UiWindowActionsPortMsg::Close);
}

#[handler]
fn minimize_window<P: UiWindowActionsPort>(this: &WindowActor<P>, _: Minimize) {
    this.port.send(UiWindowActionsPortMsg::Minimize);
}

#[handler]
fn maximize_window<P: UiWindowActionsPort>(this: &WindowActor<P>, _: Maximize) {
    this.port.send(UiWindowActionsPortMsg::ToggleMaximize);
}

#[handler]
fn resize_window<P: UiWindowActionsPort>(this: &WindowActor<P>, msg: StartResize) {
    this.port.send(UiWindowActionsPortMsg::Resize(msg.0));
}

#[handler]
fn on_breakpoint_changed<P: UiWindowActionsPort>(
    _: &WindowActor<P>,
    msg: ConfigChanged,
    ctx: &Context<WindowActor<P>>,
) {
    ctx.publish(WindowConfigChanged {
        breakpoint: msg.0,
        width: msg.1,
    });
}
