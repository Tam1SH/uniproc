use app_contracts2::features::sidebar::{SidebarBinder, SidebarReducer};
use guinea::feature::FeatureInitContext;

use super::actor::{Refresh, SetWidth, SidebarActor, Toggle};
use super::settings::SidebarSettings;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let settings = SidebarSettings::new_with(&ctx.store)?;

    let addr = ctx.spawn_actor(SidebarActor::new(ctx.port::<SidebarReducer>(), settings));

    SidebarBinder::new(&addr, &ctx.actions::<SidebarReducer>())
        .on_toggle::<Toggle>()
        .on_set_width::<SetWidth>()
        .build();

    addr.send(Refresh);

    Ok(())
}
