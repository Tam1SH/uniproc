use app_contracts::features::window_actions::{
    ResizeEdge, UiWindowActionsPort, WindowBreakpoint, WindowConfigChanged,
};
use app_core::actor::event_bus::EventBus;
use app_core::messages;
use macros::handler;

messages! {
    Drag,
    Close,
    Minimize,
    Maximize,
    Resize(ResizeEdge),
    BreakpointChanged(WindowBreakpoint, u64)
}

pub struct WindowActor<P> {
    pub port: P,
}

#[handler]
fn drag_window<P: UiWindowActionsPort>(this: &mut WindowActor<P>, _: Drag) {
    this.port.drag_window();
}

#[handler]
fn close_window<P: UiWindowActionsPort>(this: &mut WindowActor<P>, _: Close) {
    this.port.close_window();
}

#[handler]
fn minimize_window<P: UiWindowActionsPort>(this: &mut WindowActor<P>, _: Minimize) {
    this.port.minimize_window();
}

#[handler]
fn maximize_window<P: UiWindowActionsPort>(this: &mut WindowActor<P>, _: Maximize) {
    this.port.toggle_maximize_window();
}

#[handler]
fn resize_window<P: UiWindowActionsPort>(this: &mut WindowActor<P>, msg: Resize) {
    this.port.resize_window(msg.0);
}

#[handler]
fn on_breakpoint_changed<P: UiWindowActionsPort>(_: &mut WindowActor<P>, msg: BreakpointChanged) {
    EventBus::publish(WindowConfigChanged {
        breakpoint: msg.0,
        width: msg.1,
    });
}
