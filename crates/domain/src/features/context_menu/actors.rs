use app_contracts::features::context_menu::ContextMenuUiPort;
use app_core::actor::traits::{Context, Handler, Message};
use app_core::messages;
use app_core::settings::ReactiveSetting;
use slint::ComponentHandle;

messages! {
    Show { x: f32, y: f32 },
    Hide,
    HandleAction(String),
}

pub struct ContextMenuActor<P> {
    pub reveal_delay_ms: ReactiveSetting<u64>,
    pub port: P,
}

impl<TWindow, P> Handler<Show, TWindow> for ContextMenuActor<P>
where
    TWindow: ComponentHandle + 'static,
    P: ContextMenuUiPort,
{
    fn handle(&mut self, msg: Show, _ctx: &Context<Self, TWindow>) {
        self.port
            .show_menu(msg.x, msg.y, self.reveal_delay_ms.get().max(1));
        self.port.set_menu_open(true);
    }
}

impl<TWindow, P> Handler<Hide, TWindow> for ContextMenuActor<P>
where
    TWindow: ComponentHandle + 'static,
    P: ContextMenuUiPort,
{
    fn handle(&mut self, _msg: Hide, _ctx: &Context<Self, TWindow>) {
        self.port.hide_menu();
        self.port.set_menu_open(false);
    }
}

impl<TWindow, P> Handler<HandleAction, TWindow> for ContextMenuActor<P>
where
    TWindow: ComponentHandle + 'static,
    P: ContextMenuUiPort,
{
    fn handle(&mut self, msg: HandleAction, ctx: &Context<Self, TWindow>) {
        if msg.0.as_str() == "terminate" {
            self.port.invoke_terminate_selected();
        }
        ctx.addr().send(Hide);
    }
}
