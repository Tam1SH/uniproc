use app_contracts::features::sidebar::{SidebarReducer, SidebarState};
use guinea::feature::FeatureInitContext;

use super::actor::{Refresh, SidebarActor};
use super::settings::SidebarSettings;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = SidebarSettings::new()?;

    let seed = SidebarState {
        open: settings.open().get(),
        width: settings.width().get(),
    };

    ctx.seed_reducer::<SidebarReducer>(seed);

    let addr = ctx.spawn_actor(SidebarActor::new(ctx.port::<SidebarReducer>(), settings));

    ctx.wire::<SidebarReducer, _>(&addr);

    addr.send(Refresh);

    Ok(())
}
