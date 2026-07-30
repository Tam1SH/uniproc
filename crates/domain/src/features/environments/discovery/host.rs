use app_contracts::features::environments::{
    DiscoveryReport, EnvironmentDescriptor, EnvironmentKind, EnvironmentStatus,
};
use forsl_core::actor::{Context, ManagedActor};
use forsl_macros::handler;
use macros::actor_manifest;
use std::borrow::Cow;
use sysinfo::System;

#[actor_manifest]
impl ManagedActor for HostProviderActor {
    type Bus = bus!();
    type Handlers = handlers!(Init);
    type Signals = bus!(DiscoveryReport);
}

pub struct HostProviderActor;

#[handler]
fn init(_: &HostProviderActor, _: Init, ctx: &Context<HostProviderActor>) {
    let os_name = System::name().unwrap_or_else(|| {
        if cfg!(windows) {
            "Windows".into()
        } else {
            "Linux".into()
        }
    });

    let host_env = EnvironmentDescriptor {
        id: "host".into(),
        title: Cow::Owned(os_name),
        kind: EnvironmentKind::Host,
        status: EnvironmentStatus::Ready,
        capabilities: vec!["processes.list".into(), "services.list".into()],
    };

    ctx.publish(DiscoveryReport {
        provider_id: "host".into(),
        items: vec![host_env],
    });
}
