use crate::features::processes::providers::{IconProvider, NameProvider};
use crate::features::processes::SortField;
use crate::scanner::types::{ModelMachineStats, ProcessInfo};
use crate::{ProcessEntry, ProcessGroup};
use slint::{Image, ModelRc, SharedString, VecModel};
use std::collections::{HashMap, HashSet};

pub struct ProcessTreeState {
    expanded_groups: HashSet<SharedString>,
    pub all_processes: HashMap<u32, ProcessInfo>,
    show_icons: bool,
}

impl ProcessTreeState {
    pub fn new(show_icons: bool) -> Self {
        Self {
            expanded_groups: HashSet::new(),
            all_processes: HashMap::new(),
            show_icons,
        }
    }

    pub fn toggle_expand(&mut self, group_id: SharedString) {
        if self.expanded_groups.contains(&group_id) {
            self.expanded_groups.remove(&group_id);
        } else {
            self.expanded_groups.insert(group_id);
        }
    }

    pub fn build_ui_groups(
        &self,
        name_provider: &mut NameProvider,
        icon_provider: &mut IconProvider,
        stats: &ModelMachineStats,
    ) -> Vec<ProcessGroup> {
        let mut groups_map: HashMap<SharedString, Vec<u32>> = HashMap::new();

        for (pid, proc) in &self.all_processes {
            let clean_name = name_provider.get_clean(&proc.name);
            groups_map.entry(clean_name).or_default().push(*pid);
        }

        let mut result = Vec::new();

        for (group_name, mut pids) in groups_map {
            pids.sort_unstable();
            let leader_pid = pids[0];

            let is_expanded = self.expanded_groups.contains(&group_name);
            let has_children = pids.len() > 1;

            if let Some(mut parent_entry) =
                self.create_process_entry(leader_pid, 0, stats, name_provider, icon_provider)
            {
                parent_entry.has_children = has_children;
                parent_entry.is_expanded = is_expanded;
                parent_entry.name = group_name.clone();

                let mut children = Vec::new();
                if is_expanded && has_children {
                    for &child_pid in pids.iter().skip(1) {
                        if let Some(child_entry) = self.create_process_entry(
                            child_pid,
                            1,
                            stats,
                            name_provider,
                            icon_provider,
                        ) {
                            children.push(child_entry);
                        }
                    }
                }

                result.push(ProcessGroup {
                    parent: parent_entry,
                    children: ModelRc::new(VecModel::from(children)),
                });
            }
        }
        result
    }

    fn create_process_entry(
        &self,
        pid: u32,
        depth: i32,
        stats: &ModelMachineStats,
        name_provider: &mut NameProvider,
        icon_provider: &mut IconProvider,
    ) -> Option<ProcessEntry> {
        let process = self.all_processes.get(&pid)?;

        let (ram_text, ram_usage) = format_metric(process.memory_usage, false, stats.total_memory);
        let (disk_text, disk_usage) = format_metric(
            process.disk_read + process.disk_write,
            true,
            5 * 1024 * 1024,
        );
        let (net_text, net_usage) =
            format_metric(process.net_usage, true, stats.net_total_bandwidth.max(1));

        Some(ProcessEntry {
            pid: process.pid as i32,
            name: name_provider.get_clean(&process.name),
            icon: if self.show_icons {
                icon_provider.get_icon(&process.exe_path)
            } else {
                Image::default()
            },
            cpu_usage: process.cpu_usage,
            cpu_text: format!("{:.1}%", process.cpu_usage).into(),
            ram_usage,
            ram_text,
            disk_usage,
            disk_text,
            net_usage,
            net_text,
            depth,
            is_expanded: false,
            has_children: false,
            is_dead: false,
        })
    }
}

pub fn sort_processes_inplace(
    groups: &mut Vec<ProcessGroup>,
    sort_by: SortField,
    descending: bool,
) {
    groups.sort_by(|a, b| {
        let p_a = &a.parent;
        let p_b = &b.parent;

        let cmp = match sort_by {
            SortField::Cpu => p_a.cpu_usage.partial_cmp(&p_b.cpu_usage),
            SortField::Memory => p_a.ram_usage.partial_cmp(&p_b.ram_usage),
            SortField::Disk => p_a.disk_usage.partial_cmp(&p_b.disk_usage),
            SortField::Network => p_a.net_usage.partial_cmp(&p_b.net_usage),

            SortField::Name => Some(p_a.name.cmp(&p_b.name)),

            SortField::None => None,
        };

        let ord = cmp.unwrap_or(std::cmp::Ordering::Equal);

        if descending { ord.reverse() } else { ord }
    });
}

fn format_metric(val: u64, is_speed: bool, threshold: u64) -> (SharedString, f32) {
    let intensity = (val as f32 / threshold as f32 * 100.0).clamp(0.0, 100.0);

    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    let text = if val >= GB {
        format!("{:.1} GiB", val as f64 / GB as f64)
    } else if val >= MB {
        format!("{:.0} MiB", val as f64 / MB as f64)
    } else {
        format!("{:.0} KiB", val as f64 / KB as f64)
    };

    let suffix = if is_speed { "/s" } else { "" };
    (format!("{}{}", text, suffix).into(), intensity)
}
