use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::navigation::KnownRouteDescriptor;
use app_contracts::features::tabs::{
    AvailableContextDescriptor, CapabilityDescriptor, CapabilityStatus, TabContextKey,
    TabContextKind, TabContextSnapshot, TabDescriptor, TabPageDescriptor,
};
use context::page_status::PageStatus;
use framework::navigation::RouteRegistry;
use std::borrow::Cow;
use sysinfo::System;
use uniproc_protocol::LinuxEnvironmentKind;

pub fn bootstrap_contexts() -> Vec<TabContextSnapshot> {
    vec![TabContextSnapshot {
        key: TabContextKey::HOST,
        kind: TabContextKind::Host,
        title: System::name().unwrap_or_else(|| "Windows".into()),
        icon_key: "windows-11".into(),
        capabilities: vec![
            capability("processes.list", "Processes"),
            capability("services.list", "Services"),
        ],
        status: PageStatus::Ready,
        ..Default::default()
    }]
}

pub fn build_tabs(contexts: &[TabContextSnapshot], routes: &RouteRegistry) -> Vec<TabDescriptor> {
    contexts
        .iter()
        .map(|context| TabDescriptor {
            context_key: context.key.clone(),
            title: context.title.clone(),
            icon_key: context.icon_key.clone(),
            pages: project_pages(context, routes),
            status: context.status,
            error_msg: context.error_msg.clone(),
            is_closable: !matches!(context.kind, TabContextKind::Host),
        })
        .collect()
}

pub fn build_available_contexts(
    contexts: &[TabContextSnapshot],
    enabled_contexts: &std::collections::HashSet<TabContextKey>,
) -> Vec<AvailableContextDescriptor> {
    contexts
        .iter()
        .filter(|context| !enabled_contexts.contains(&context.key))
        .map(|context| AvailableContextDescriptor {
            context_key: context.key.clone(),
            title: context.title.clone(),
            icon_key: context.icon_key.clone(),
            status: context.status,
        })
        .collect()
}

pub fn update_context_status(
    contexts: &mut [TabContextSnapshot],
    context_key: &str,
    status: PageStatus,
) -> bool {
    if let Some(context) = contexts
        .iter_mut()
        .find(|context| context.key.0 == context_key)
    {
        if context.status != status {
            context.status = status;
            return true;
        }
    }

    false
}

pub fn default_enabled_context_keys(contexts: &[TabContextSnapshot]) -> Vec<TabContextKey> {
    contexts
        .iter()
        .filter(|context| matches!(context.kind, TabContextKind::Host))
        .map(|context| context.key.clone())
        .collect()
}

pub fn apply_remote_contexts(
    contexts: &mut Vec<TabContextSnapshot>,
    report: &RemoteScanResult,
) -> bool {
    let mut changed = false;
    let dynamic_prefix = match report.schema_id {
        "wsl" => "wsl",
        "linux" => "linux",
        _ => return false,
    };

    let mut next_dynamic = Vec::new();

    for environment in &report.environments {
        if let LinuxEnvironmentKind::CurrentDistro { name } = &environment.kind {
            next_dynamic.push(TabContextSnapshot {
                key: TabContextKey(Cow::Owned(format!("{dynamic_prefix}/distro/{name}"))),
                kind: TabContextKind::Wsl,
                title: name.clone(),
                icon_key: icon_for_env_name(name).into(),
                capabilities: vec![
                    capability("processes.list", "Processes"),
                    capability("agent.shell", "Shell"),
                ],
                status: PageStatus::Ready,
                ..Default::default()
            });
        }
    }

    for container in &report.docker_containers {
        let short_id: String = container.id.chars().take(12).collect();
        next_dynamic.push(TabContextSnapshot {
            key: TabContextKey(Cow::Owned(format!(
                "{dynamic_prefix}/docker/{}",
                container.id
            ))),
            kind: TabContextKind::Docker,
            title: format!("Docker {short_id}"),
            icon_key: "docker".into(),
            capabilities: vec![capability("processes.list", "Processes")],
            status: PageStatus::Ready,
            ..Default::default()
        });
    }

    let previous_len = contexts.len();
    contexts.retain(|context| !is_dynamic_context_for(context, dynamic_prefix));
    if contexts.len() != previous_len {
        changed = true;
    }

    for snapshot in next_dynamic {
        if !contexts.iter().any(|context| context.key == snapshot.key) {
            changed = true;
        }
        contexts.push(snapshot);
    }

    changed
}

fn project_pages(context: &TabContextSnapshot, routes: &RouteRegistry) -> Vec<TabPageDescriptor> {
    let mut pages = Vec::new();

    if has_capability(context, "processes.list") {
        if let Some(page) =
            page_descriptor(routes, &context.key, "processes", "Processes", "apps-list")
        {
            pages.push(page);
        }
    }

    if has_capability(context, "services.list") {
        if let Some(page) = page_descriptor(routes, &context.key, "services", "Services", "puzzle")
        {
            pages.push(page);
        }
    }

    if has_capability(context, "disk.overview") {
        if let Some(page) = page_descriptor(routes, &context.key, "disk", "Disk", "disk") {
            pages.push(page);
        }
    }

    pages
}

pub fn navigation_routes(tabs: &[TabDescriptor]) -> Vec<KnownRouteDescriptor> {
    vec![]
    // tabs.iter()
    //     .flat_map(|tab| {
    //         tab.pages.iter().map(|page| KnownRouteDescriptor {
    //             path: Cow::from(page.path.clone()),
    //             segment: Cow::from(page.route_segment.clone()),
    //             features: vec![],
    //         })
    //     })
    //     .collect()
}

fn page_descriptor(
    routes: &RouteRegistry,
    context_key: &TabContextKey,
    page_key: &str,
    text: &str,
    icon_key: &str,
) -> Option<TabPageDescriptor> {
    None
    // let route = routes.find_by_page_key(page_key)?;
    //
    // Some(TabPageDescriptor {
    //     path: canonical_route_path(context_key, &route.route_segment),
    //     route_segment: route.route_segment.to_string(),
    //     text: text.to_string(),
    //     icon_key: icon_key.to_string(),
    //     ..Default::default()
    // })
}

fn has_capability(context: &TabContextSnapshot, capability_id: &str) -> bool {
    context
        .capabilities
        .iter()
        .any(|cap| cap.id == capability_id && cap.status != CapabilityStatus::Unavailable)
}

fn is_dynamic_context_for(context: &TabContextSnapshot, prefix: &str) -> bool {
    context.key.0.starts_with(&format!("{prefix}/distro/"))
        || context.key.0.starts_with(&format!("{prefix}/docker/"))
}

fn icon_for_env_name(name: &str) -> &'static str {
    let name_low = name.to_lowercase();

    match () {
        _ if name_low.contains("ubuntu") => "ubuntu",
        _ if name_low.contains("docker") => "docker",
        _ => "linux",
    }
}

fn capability(id: &str, title: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: id.into(),
        title: title.into(),
        ..Default::default()
    }
}
