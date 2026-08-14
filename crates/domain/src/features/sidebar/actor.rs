use app_contracts::features::sidebar::{SetOpen, SetWidth, SidebarMsg, SidebarPort, Toggle};
use guinea_core::actor::Context;
use guinea_core::messages;
use guinea_macros::{actor, handler};

use super::settings::SidebarSettings;

messages! { Refresh }

#[derive(derive_more::Debug)]
pub struct SidebarActor<P: SidebarPort> {
    #[debug(skip)]
    ui_port: P,
    settings: SidebarSettings,
}

impl<P: SidebarPort> SidebarActor<P> {
    pub fn new(ui_port: P, settings: SidebarSettings) -> Self {
        Self { ui_port, settings }
    }

    fn publish(&self) {
        let open = self.settings.open().get();
        let width = self.settings.width().get();
        tracing::debug!(open, width, "sidebar publish");
        self.ui_port.send(SidebarMsg::Set { open, width });
    }
}

actor! {
    SidebarActor<P: SidebarPort> {
        handlers { Toggle, SetOpen, SetWidth, Refresh }
    }
}

#[handler]
fn toggle<P: SidebarPort>(
    this: &mut SidebarActor<P>,
    _ctx: Context<SidebarActor<P>, Toggle>,
) {
    let open = !this.settings.open().get();
    let _ = this.settings.open().set(open);
    this.publish();
}

#[handler]
fn set_open<P: SidebarPort>(
    this: &mut SidebarActor<P>,
    ctx: Context<SidebarActor<P>, SetOpen>,
) {
    let _ = this.settings.open().set(ctx.msg.0);
    this.publish();
}

#[handler]
fn set_width<P: SidebarPort>(
    this: &mut SidebarActor<P>,
    ctx: Context<SidebarActor<P>, SetWidth>,
) {
    let _ = this.settings.width().set(ctx.msg.0);
    this.publish();
}

#[handler]
fn refresh<P: SidebarPort>(
    this: &SidebarActor<P>,
    _ctx: Context<SidebarActor<P>, Refresh>,
) {
    this.publish();
}
