use app_contracts2::features::processes::{ProcessCategory, ProcessRow};

pub(super) const MEMORY_COMPRESSION: &str = "Memory Compression";
use std::collections::HashMap;
use std::collections::HashSet;

pub(super) struct ProcessGroup {
    leader: ProcessRow,
    members: Vec<ProcessRow>,
}

pub(super) fn group_by_name(rows: &[ProcessRow], leader_pid: Option<u32>) -> Vec<ProcessGroup> {
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<ProcessRow>> = HashMap::new();
    for row in rows {
        groups
            .entry(row.name.clone())
            .or_insert_with(|| {
                order.push(row.name.clone());
                Vec::new()
            })
            .push(row.clone());
    }

    order
        .into_iter()
        .filter_map(|name| {
            let mut members = groups.remove(&name)?;
            if members.is_empty() {
                return None;
            }
            members.sort_unstable_by_key(|r| r.pid);
            let leader_idx = leader_pid
                .and_then(|pid| members.iter().position(|r| r.pid == pid))
                .unwrap_or(0);
            members.swap(0, leader_idx);

            let leader = if members.len() > 1 {
                ProcessRow {
                    cpu_percent: members.iter().map(|r| r.cpu_percent).sum(),
                    memory_bytes: members.iter().map(|r| r.memory_bytes).sum(),
                    disk_bytes: members.iter().map(|r| r.disk_bytes).sum(),
                    net_bytes: members.iter().map(|r| r.net_bytes).sum(),
                    ..members[0].clone()
                }
            } else {
                members[0].clone()
            };

            Some(ProcessGroup { leader, members })
        })
        .collect()
}

pub(super) struct Section {
    pub(super) category: ProcessCategory,
    groups: Vec<ProcessGroup>,
}

pub(super) fn split_by_category(groups: Vec<ProcessGroup>) -> Vec<Section> {
    let mut by_category: HashMap<ProcessCategory, Vec<ProcessGroup>> = HashMap::new();
    for group in groups {
        by_category.entry(group_category(&group)).or_default().push(group);
    }

    ProcessCategory::ORDER
        .into_iter()
        .filter_map(|category| {
            let groups = by_category.remove(&category)?;
            (!groups.is_empty()).then_some(Section { category, groups })
        })
        .collect()
}

fn group_category(group: &ProcessGroup) -> ProcessCategory {
    let rank = |c: ProcessCategory| {
        ProcessCategory::ORDER
            .iter()
            .position(|o| *o == c)
            .unwrap_or(usize::MAX)
    };
    group
        .members
        .iter()
        .map(|m| m.category)
        .min_by_key(|c| rank(*c))
        .unwrap_or(group.leader.category)
}

#[derive(Default, Clone, Copy)]
struct SectionTotals {
    cpu_percent: f32,
    memory_bytes: u64,
    disk_bytes: u64,
    net_bytes: u64,
    group_count: usize,
    compressed_bytes: u64,
}

impl SectionTotals {
    fn of<'a>(groups: impl Iterator<Item = &'a ProcessGroup>) -> Self {
        let mut totals = Self::default();
        for group in groups {
            totals.cpu_percent += group.leader.cpu_percent;
            totals.memory_bytes += group.leader.memory_bytes;
            totals.disk_bytes += group.leader.disk_bytes;
            totals.net_bytes += group.leader.net_bytes;
            totals.group_count += 1;

            if group.leader.name == MEMORY_COMPRESSION {
                totals.compressed_bytes += group.leader.memory_bytes;
            }
        }
        totals
    }
}

pub(super) fn flatten_for_display(
    sections: &[Section],
    expanded: &HashSet<String>,
    collapsed_sections: &HashSet<ProcessCategory>,
) -> Vec<DisplayRow> {
    let mut out = Vec::new();

    for section in sections {
        let totals = SectionTotals::of(section.groups.iter());
        let section_expanded = !collapsed_sections.contains(&section.category);
        out.push(DisplayRow::section(
            section.category,
            totals,
            section_expanded,
        ));

        if !section_expanded {
            continue;
        }

        for group in &section.groups {
            let has_children = group.members.len() > 1;
            let is_expanded = has_children && expanded.contains(&group.leader.name);

            out.push(DisplayRow {
                row: group.leader.clone(),
                depth: 1,
                has_children,
                is_expanded,
                group_size: group.members.len(),
                section: None,
            });

            if is_expanded {
                out.extend(group.members[1..].iter().map(|r| DisplayRow {
                    row: r.clone(),
                    depth: 2,
                    has_children: false,
                    is_expanded: false,
                    group_size: 1,
                    section: None,
                }));
            }
        }
    }
    out
}

pub(super) struct GroupsCache {
    rows_ptr: *const ProcessRow,
    rows_len: usize,
    selected: Option<u32>,
    sections: Vec<Section>,
}

impl GroupsCache {
    pub(super) fn empty() -> Self {
        Self {
            rows_ptr: std::ptr::null(),
            rows_len: 0,
            selected: None,
            sections: Vec::new(),
        }
    }

    pub(super) fn get(&mut self, rows: &[ProcessRow], selected: Option<u32>) -> &[Section] {
        let rows_ptr = rows.as_ptr();
        if self.rows_ptr != rows_ptr || self.rows_len != rows.len() || self.selected != selected {
            self.sections = split_by_category(group_by_name(rows, selected));
            self.rows_ptr = rows_ptr;
            self.rows_len = rows.len();
            self.selected = selected;
        }
        &self.sections
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct SectionRow {
    pub(super) category: ProcessCategory,
    pub(super) compressed_bytes: u64,
}

#[derive(Clone)]
pub(super) struct DisplayRow {
    pub(super) row: ProcessRow,
    pub(super) depth: u8,
    pub(super) has_children: bool,
    pub(super) is_expanded: bool,
    pub(super) group_size: usize,
    pub(super) section: Option<SectionRow>,
}

impl DisplayRow {
    fn section(category: ProcessCategory, totals: SectionTotals, is_expanded: bool) -> Self {
        let label = category.label();
        Self {
            row: ProcessRow {
                pid: 0,
                name: label.to_string(),
                display_name: label.to_string(),
                cpu_percent: totals.cpu_percent,
                memory_bytes: totals.memory_bytes.saturating_sub(totals.compressed_bytes),
                disk_bytes: totals.disk_bytes,
                net_bytes: totals.net_bytes,
                exe_path: String::new(),
                package_full_name: String::new(),
                category: ProcessCategory::App,
            },
            depth: 0,
            has_children: totals.group_count > 0,
            is_expanded,
            group_size: totals.group_count,
            section: Some(SectionRow {
                category,
                compressed_bytes: totals.compressed_bytes,
            }),
        }
    }
}

pub(super) fn pin_display_row(
    rows: &mut Vec<DisplayRow>,
    target_pid: Option<u32>,
    prev_pos: Option<usize>,
) -> Option<usize> {
    let pid = target_pid?;
    let current = rows.iter().position(|d| d.row.pid == pid)?;
    let insert_at = prev_pos.unwrap_or(current).min(rows.len() - 1);
    if insert_at != current {
        let row = rows.remove(current);
        rows.insert(insert_at, row);
    }
    Some(insert_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, name: &str) -> ProcessRow {
        categorised(pid, name, ProcessCategory::App)
    }

    fn categorised(pid: u32, name: &str, category: ProcessCategory) -> ProcessRow {
        ProcessRow {
            pid,
            name: name.to_string(),
            display_name: name.to_string(),
            cpu_percent: pid as f32,
            memory_bytes: 0,
            disk_bytes: 0,
            net_bytes: 0,
            exe_path: String::new(),
            package_full_name: String::new(),
            category,
        }
    }

    fn process_pids(rows: &[DisplayRow]) -> Vec<u32> {
        rows.iter()
            .filter(|d| d.section.is_none())
            .map(|d| d.row.pid)
            .collect()
    }

    fn labels(rows: &[DisplayRow]) -> Vec<&str> {
        rows.iter()
            .filter_map(|d| d.section.as_ref().map(|s| s.category.label()))
            .collect()
    }

    fn one_section(groups: Vec<ProcessGroup>) -> Vec<Section> {
        split_by_category(groups)
    }

    fn pids(rows: &[DisplayRow]) -> Vec<u32> {
        rows.iter().map(|d| d.row.pid).collect()
    }

    fn group(leader: ProcessRow, rest: Vec<ProcessRow>) -> ProcessGroup {
        let mut members = vec![leader.clone()];
        members.extend(rest);
        ProcessGroup { leader, members }
    }

    #[test]
    fn ungrouped_rows_form_singleton_groups() {
        let rows = vec![row(1, "alpha"), row(2, "beta"), row(3, "gamma")];
        let groups = group_by_name(&rows, None);

        assert_eq!(groups.len(), 3);
        assert!(groups.iter().all(|g| g.members.len() == 1));
    }

    #[test]
    fn duplicate_names_group_with_lowest_pid_as_leader_by_default() {
        let rows = vec![
            row(5, "chrome.exe"),
            row(2, "chrome.exe"),
            row(9, "chrome.exe"),
        ];
        let groups = group_by_name(&rows, None);

        assert_eq!(groups.len(), 1, "same-named rows collapse into one group");
        assert_eq!(groups[0].leader.pid, 2, "leader defaults to the lowest pid");
        assert_eq!(groups[0].members.len(), 3);
    }

    #[test]
    fn selecting_a_non_lowest_pid_member_promotes_it_to_leader() {
        let rows = vec![
            row(5, "chrome.exe"),
            row(2, "chrome.exe"),
            row(9, "chrome.exe"),
        ];
        let groups = group_by_name(&rows, Some(9));

        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].leader.pid, 9,
            "selected member becomes the leader, not the lowest pid"
        );
    }

    #[test]
    fn leader_sums_metrics_across_the_group() {
        let rows = vec![row(1, "chrome.exe"), row(2, "chrome.exe")];
        let groups = group_by_name(&rows, None);

        assert_eq!(groups[0].leader.cpu_percent, 1.0 + 2.0);
    }

    #[test]
    fn collapsed_groups_render_as_a_single_leader_row() {
        let sections = one_section(vec![group(
            row(2, "chrome.exe"),
            vec![row(5, "chrome.exe"), row(9, "chrome.exe")],
        )]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());

        assert_eq!(process_pids(&out), vec![2]);
        let leader = out.iter().find(|d| d.section.is_none()).unwrap();
        assert!(leader.has_children);
        assert_eq!(leader.group_size, 3);
    }

    #[test]
    fn expanding_a_group_inserts_members_right_after_its_leader() {
        let sections = one_section(vec![
            group(row(10, "alpha"), vec![]),
            group(row(2, "chrome.exe"), vec![row(5, "chrome.exe")]),
            group(row(20, "zeta"), vec![]),
        ]);
        let mut expanded = HashSet::new();
        expanded.insert("chrome.exe".to_string());
        let out = flatten_for_display(&sections, &expanded, &HashSet::new());

        assert_eq!(process_pids(&out), vec![10, 2, 5, 20]);
        let member = out.iter().find(|d| d.row.pid == 5).unwrap();
        assert_eq!(member.depth, 2, "a group member sits one below its leader");
        assert!(!member.has_children);
    }

    #[test]
    fn categories_render_in_order_under_their_parent_headings() {
        let sections = split_by_category(vec![
            group(categorised(1, "svchost.exe", ProcessCategory::WindowsService), vec![]),
            group(categorised(2, "chrome.exe", ProcessCategory::App), vec![]),
            group(categorised(3, "updater.exe", ProcessCategory::BackgroundThirdParty), vec![]),
            group(categorised(4, "System", ProcessCategory::WindowsKernel), vec![]),
            group(categorised(5, "RuntimeBroker.exe", ProcessCategory::BackgroundMicrosoft), vec![]),
        ]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());

        assert_eq!(
            labels(&out),
            vec![
                "Apps",
                "Background processes",
                "Background processes (Microsoft)",
                "Services",
                "Windows kernel",
            ]
        );
        assert_eq!(process_pids(&out), vec![2, 3, 5, 1, 4]);
    }

    #[test]
    fn an_empty_category_gets_no_heading() {
        let sections = split_by_category(vec![group(
            categorised(1, "chrome.exe", ProcessCategory::App),
            vec![],
        )]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());

        assert_eq!(labels(&out), vec!["Apps"]);
    }

    #[test]
    fn collapsing_a_category_hides_its_processes_but_keeps_the_heading() {
        let sections = split_by_category(vec![
            group(categorised(1, "chrome.exe", ProcessCategory::App), vec![]),
            group(categorised(2, "svchost.exe", ProcessCategory::WindowsService), vec![]),
        ]);
        let mut collapsed = HashSet::new();
        collapsed.insert(ProcessCategory::WindowsService);
        let out = flatten_for_display(&sections, &HashSet::new(), &collapsed);

        assert_eq!(process_pids(&out), vec![1], "the service row is hidden");
        assert!(
            labels(&out).contains(&"Services"),
            "its heading stays"
        );
    }

    #[test]
    fn the_kernel_heading_takes_compressed_memory_out_of_its_total() {
        let mut compression = categorised(4, MEMORY_COMPRESSION, ProcessCategory::WindowsKernel);
        compression.memory_bytes = 3_000;
        let mut system = categorised(5, "System", ProcessCategory::WindowsKernel);
        system.memory_bytes = 1_000;

        let sections = split_by_category(vec![group(compression, vec![]), group(system, vec![])]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());
        let heading = out.iter().find(|d| d.section.is_some()).unwrap();

        assert_eq!(heading.row.memory_bytes, 1_000);
        assert_eq!(heading.section.as_ref().unwrap().compressed_bytes, 3_000);
    }

    #[test]
    fn other_sections_report_no_compression() {
        let sections = split_by_category(vec![group(
            categorised(1, "chrome.exe", ProcessCategory::App),
            vec![],
        )]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());
        let heading = out.iter().find(|d| d.section.is_some()).unwrap();

        assert_eq!(heading.section.as_ref().unwrap().compressed_bytes, 0);
    }

    #[test]
    fn a_heading_counts_groups_not_processes() {
        let sections = split_by_category(vec![
            group(
                categorised(1, "chrome.exe", ProcessCategory::App),
                vec![
                    categorised(2, "chrome.exe", ProcessCategory::BackgroundThirdParty),
                    categorised(3, "chrome.exe", ProcessCategory::BackgroundThirdParty),
                ],
            ),
            group(categorised(4, "code.exe", ProcessCategory::App), vec![]),
        ]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());

        let heading = out.iter().find(|d| d.section.is_some()).unwrap();
        assert_eq!(heading.group_size, 2, "two apps, not four processes");
    }

    #[test]
    fn a_group_is_an_app_if_any_member_owns_a_window() {
        let sections = split_by_category(vec![group(
            categorised(1, "chrome.exe", ProcessCategory::BackgroundThirdParty),
            vec![
                categorised(2, "chrome.exe", ProcessCategory::BackgroundThirdParty),
                categorised(3, "chrome.exe", ProcessCategory::App),
            ],
        )]);

        assert_eq!(labels(&flatten_for_display(
            &sections,
            &HashSet::new(),
            &HashSet::new()
        )), vec!["Apps"]);
    }

    #[test]
    fn a_heading_can_never_be_selected() {
        let sections = split_by_category(vec![group(
            categorised(1, "chrome.exe", ProcessCategory::App),
            vec![],
        )]);
        let out = flatten_for_display(&sections, &HashSet::new(), &HashSet::new());

        assert!(out.iter().filter(|d| d.section.is_some()).all(|d| d.row.pid == 0));
    }

    #[test]
    fn pin_keeps_the_selected_row_at_its_previous_position_across_a_resort() {
        let mut rows = vec![row(1, "a"), row(2, "b"), row(3, "c")];
        let display: Vec<DisplayRow> = rows
            .drain(..)
            .map(|row| DisplayRow {
                row,
                depth: 0,
                has_children: false,
                is_expanded: false,
                group_size: 1,
                section: None,
            })
            .collect();

        let mut first = display.clone();
        let pos = pin_display_row(&mut first, Some(2), None);
        assert_eq!(pos, Some(1));
        assert_eq!(
            pids(&first),
            vec![1, 2, 3],
            "no prior position - nothing should move yet"
        );

        let mut resorted = vec![display[1].clone(), display[0].clone(), display[2].clone()];
        let pos = pin_display_row(&mut resorted, Some(2), pos);
        assert_eq!(pos, Some(1));
        assert_eq!(
            pids(&resorted),
            vec![1, 2, 3],
            "pinned row pulled back to index 1"
        );
    }

    #[test]
    fn pin_reports_none_when_the_target_is_not_currently_rendered() {
        let mut display = vec![DisplayRow {
            row: row(1, "a"),
            depth: 0,
            has_children: false,
            is_expanded: false,
            group_size: 1,
            section: None,
        }];
        let pos = pin_display_row(&mut display, Some(99), Some(0));
        assert_eq!(pos, None);
    }
}
