use crate::features::environments::wsl::domain::fetch_distros_data;
use app_contracts::features::environments::{
    DiscoveryReport, EnvironmentDescriptor, EnvironmentKind, EnvironmentStatus,
};
use forsl_core::actor::{AsyncContext, Context, ManagedActor};
use forsl_macros::handler;
use macros::actor_manifest;

#[actor_manifest]
impl ManagedActor for WslDiscoveryActor {
    type Bus = bus!();
    type Handlers = handlers!(Init, Refresh);
    type Signals = bus!(DiscoveryReport);
}

pub struct WslDiscoveryActor;

#[handler]
fn init(_: &WslDiscoveryActor, _: Init, ctx: &Context<WslDiscoveryActor>) {
    ctx.addr().send(Refresh);
}

#[handler]
async fn handle_refresh(ctx: AsyncContext<WslDiscoveryActor>, _: Refresh) {
    let raw_data = fetch_distros_data().await;

    let items = raw_data
        .into_iter()
        .map(|d| EnvironmentDescriptor {
            id: format!("wsl://{}", d.name).into(),
            title: d.name.into(),
            kind: EnvironmentKind::Wsl,
            status: if d.is_running {
                EnvironmentStatus::Ready
            } else {
                EnvironmentStatus::Degraded
            },
            capabilities: vec!["processes.list".into()],
        })
        .collect();

    ctx.publish(DiscoveryReport {
        provider_id: "wsl".into(),
        items,
    });
}
