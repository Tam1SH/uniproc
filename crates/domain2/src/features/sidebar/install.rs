use app_contracts2::features::sidebar::{SidebarBinder, SidebarReducer, SidebarState};
use guinea::feature::FeatureInitContext;

use super::actor::{Refresh, SetOpen, SetWidth, SidebarActor, Toggle};
use super::settings::SidebarSettings;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = SidebarSettings::new_with(&ctx.store)?;

    let seed = SidebarState {
        open: settings.open().get(),
        width: settings.width().get(),
    };

    ctx.seed_reducer::<SidebarReducer>(seed);

    let addr = ctx.spawn_actor(SidebarActor::new(ctx.port::<SidebarReducer>(), settings));

    SidebarBinder::new(&addr, &ctx.actions::<SidebarReducer>())
        .on_toggle::<Toggle>()
        .on_set_open::<SetOpen>()
        .on_set_width::<SetWidth>()
        .build();

    addr.send(Refresh);

    Ok(())
}
