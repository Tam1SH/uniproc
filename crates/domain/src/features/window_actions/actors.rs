use app_contracts::features::window_actions::{
    ResizeEdge, UiWindowActionsPort, WindowBreakpoint, WindowConfigChanged,
};
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;
use app_core::messages;

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

impl<TWindow, P> Handler<Drag, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, _msg: Drag, _ctx: &Context<Self, TWindow>) {
        self.port.drag_window();
    }
}

impl<TWindow, P> Handler<Close, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, _msg: Close, _ctx: &Context<Self, TWindow>) {
        self.port.close_window();
    }
}

impl<TWindow, P> Handler<Minimize, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, _msg: Minimize, _ctx: &Context<Self, TWindow>) {
        self.port.minimize_window();
    }
}

impl<TWindow, P> Handler<Maximize, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, _msg: Maximize, _ctx: &Context<Self, TWindow>) {
        self.port.toggle_maximize_window();
    }
}

impl<TWindow, P> Handler<Resize, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, msg: Resize, _ctx: &Context<Self, TWindow>) {
        self.port.resize_window(msg.0);
    }
}

impl<TWindow, P> Handler<BreakpointChanged, TWindow> for WindowActor<P>
where
    TWindow: Window,
    P: UiWindowActionsPort,
{
    fn handle(&mut self, msg: BreakpointChanged, _ctx: &Context<Self, TWindow>) {
        EventBus::publish(WindowConfigChanged {
            breakpoint: msg.0,
            width: msg.1,
        });
    }
}
