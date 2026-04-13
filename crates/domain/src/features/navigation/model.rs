use app_contracts::features::agents::RemoteScanResult;
use app_contracts::features::navigation::{
    page_ids, CapabilityDescriptor, CapabilityStatus, PageDescriptor, TabContextKey,
    TabContextKind, TabContextSnapshot, TabDescriptor,
};
use context::page_status::PageStatus;
use sysinfo::System;
use uniproc_protocol::LinuxEnvironmentKind;

pub fn bootstrap_contexts() -> Vec<TabContextSnapshot> {
    vec![
        TabContextSnapshot {
            key: TabContextKey("host/windows".into()),
            kind: TabContextKind::Host,
            title: System::name().unwrap_or_else(|| "Windows".into()),
            icon_key: "windows-11".into(),
            capabilities: vec![
                capability("processes.list", "Processes"),
                capability("services.list", "Services"),
                capability("disk.overview", "Disk"),
            ],
            status: PageStatus::Ready,
            ..Default::default()
        },
    ]
}

pub fn build_tabs(contexts: &[TabContextSnapshot]) -> Vec<TabDescriptor> {
    contexts
        .iter()
        .map(|context| TabDescriptor {
            id: tab_id_for_context(context),
            context_key: context.key.clone(),
            title: context.title.clone(),
            icon_key: context.icon_key.clone(),
            pages: project_pages(context),
            status: context.status,
            error_msg: context.error_msg.clone(),
            is_closable: !matches!(context.kind, TabContextKind::Host),
        })
        .collect()
}

pub fn update_context_status(
    contexts: &mut [TabContextSnapshot],
    context_key: &str,
    status: PageStatus,
) -> bool {
    if let Some(context) = contexts.iter_mut().find(|context| context.key.0 == context_key) {
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
                key: TabContextKey(format!("{dynamic_prefix}/distro/{name}")),
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
            key: TabContextKey(format!("{dynamic_prefix}/docker/{}", container.id)),
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

fn tab_id_for_context(context: &TabContextSnapshot) -> context::page_status::TabId {
    let mut hash: u32 = 2_166_136_261;
    for byte in context.key.0.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }

    context::page_status::TabId(hash.max(1))
}

fn project_pages(context: &TabContextSnapshot) -> Vec<PageDescriptor> {
    let mut pages = Vec::new();

    if has_capability(context, "processes.list") {
        pages.push(PageDescriptor {
            id: page_ids::PROCESSES,
            text: "Processes".into(),
            icon_key: "apps-list".into(),
            ..Default::default()
        });
    }

    if has_capability(context, "services.list") {
        pages.push(PageDescriptor {
            id: page_ids::SERVICES,
            text: "Services".into(),
            icon_key: "puzzle".into(),
            ..Default::default()
        });
    }

    if has_capability(context, "disk.overview") {
        pages.push(PageDescriptor {
            id: page_ids::DISK,
            text: "Disk".into(),
            icon_key: "disk".into(),
            ..Default::default()
        });
    }

    pages
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
