use app_contracts::features::sidebar::{SidebarMsg, SidebarPort};
use guinea_core::actor::ManagedActor;
use guinea_macros::{actor_manifest, handler};

use super::settings::SidebarSettings;

#[derive(derive_more::Debug)]
pub struct SidebarActor<P: SidebarPort> {
    #[debug(skip)]
    ui_port: P,
    #[debug(skip)]
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

#[actor_manifest]
impl<P: SidebarPort> ManagedActor for SidebarActor<P> {
    type Handlers = handlers!(
        bind {
            Toggle,
            SetOpen(bool),
            SetWidth(u64)
        },
        Refresh
    );
}

#[handler]
fn toggle<P: SidebarPort>(this: &mut SidebarActor<P>, _: Toggle) {
    let open = !this.settings.open().get();
    let _ = this.settings.open().set(open);
    this.publish();
}

#[handler]
fn set_open<P: SidebarPort>(this: &mut SidebarActor<P>, SetOpen(open): SetOpen) {
    let _ = this.settings.open().set(open);
    this.publish();
}

#[handler]
fn set_width<P: SidebarPort>(this: &mut SidebarActor<P>, SetWidth(width): SetWidth) {
    let _ = this.settings.width().set(width);
    this.publish();
}

#[handler]
fn refresh<P: SidebarPort>(this: &SidebarActor<P>, _: Refresh) {
    this.publish();
}
