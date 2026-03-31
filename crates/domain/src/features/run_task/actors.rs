use app_contracts::features::run_task::{RunTaskPort, RunTaskRequest};
use app_core::actor::traits::{Context, Handler, Message};
use app_core::app::Window;
use app_core::messages;

messages! {
    Show,
    Hide,
    Drag,
    Execute(RunTaskRequest)
}

pub struct RunTaskActor<P> {
    pub port: P,
}

impl<TWindow, P> Handler<Drag, TWindow> for RunTaskActor<P>
where
    TWindow: Window,
    P: RunTaskPort,
{
    fn handle(&mut self, _msg: Drag, _ctx: &Context<Self, TWindow>) {
        self.port.drag_dialog_window();
    }
}

impl<TWindow, P> Handler<Show, TWindow> for RunTaskActor<P>
where
    TWindow: Window,
    P: RunTaskPort,
{
    fn handle(&mut self, _msg: Show, _ctx: &Context<Self, TWindow>) {
        self.port.show_dialog();
    }
}

impl<TWindow, P> Handler<Hide, TWindow> for RunTaskActor<P>
where
    TWindow: Window,
    P: RunTaskPort,
{
    fn handle(&mut self, _msg: Hide, _ctx: &Context<Self, TWindow>) {
        self.port.hide_dialog();
    }
}

impl<TWindow, P> Handler<Execute, TWindow> for RunTaskActor<P>
where
    TWindow: Window,
    P: RunTaskPort,
{
    fn handle(&mut self, _msg: Execute, ctx: &Context<Self, TWindow>) {
        ctx.addr().send(Hide);
    }
}
