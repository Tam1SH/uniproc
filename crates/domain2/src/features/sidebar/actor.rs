use app_contracts2::features::sidebar::{SidebarMsg, SidebarPort};
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};

use super::settings::SidebarSettings;

pub struct SidebarActor<P: SidebarPort> {
    ui_port: P,
    open: bool,
    settings: SidebarSettings,
}

impl<P: SidebarPort> std::fmt::Debug for SidebarActor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidebarActor")
            .field("open", &self.open)
            .field("width", &self.settings.width().get())
            .finish()
    }
}

impl<P: SidebarPort> SidebarActor<P> {
    pub fn new(ui_port: P, settings: SidebarSettings) -> Self {
        Self { ui_port, open: true, settings }
    }

    fn publish(&self) {
        self.ui_port.send(SidebarMsg::Set { open: self.open, width: self.settings.width().get() });
    }
}

#[actor_manifest]
impl<P: SidebarPort> ManagedActor for SidebarActor<P> {
    type Handlers = handlers!(
        bind {
            Toggle,
            SetWidth(u64)
        },
        Refresh
    );
}

#[handler]
fn toggle<P: SidebarPort>(this: &mut SidebarActor<P>, _: Toggle) {
    this.open = !this.open;
    this.publish();
}

#[handler]
fn set_width<P: SidebarPort>(this: &mut SidebarActor<P>, msg: SetWidth) {
    let _ = this.settings.width().set(msg.0);
    this.publish();
}

#[handler]
fn refresh<P: SidebarPort>(this: &SidebarActor<P>, _: Refresh) {
    this.publish();
}
