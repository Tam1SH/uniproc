use app_contracts2::features::agents::WindowsReportMessage;
use app_contracts2::features::metrics::MetricsReducer;
use guinea::feature::FeatureInitContext;

use super::actor::MetricsActor;

pub fn install(ctx: &FeatureInitContext) -> anyhow::Result<()> {
    let addr = ctx.spawn_actor(MetricsActor::new(ctx.port::<MetricsReducer>()));
    ctx.subscribe_on_global_bus::<MetricsActor<_>, WindowsReportMessage>(addr);
    Ok(())
}
