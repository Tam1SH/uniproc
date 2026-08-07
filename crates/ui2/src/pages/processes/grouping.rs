use app_contracts2::features::processes::ProcessRow;
use std::collections::HashMap;
use std::collections::HashSet;

/// A family of same-named processes with their metrics summed - a fact
/// about the system, independent of how (or whether) it gets rendered as a
/// collapsible row. No `depth`/`is_expanded`/chevron concept belongs here;
/// those exist only because of how a table widget draws a group, which is
/// [`flatten_for_display`]'s job, not this one's.
pub(super) struct ProcessGroup {
    /// Identity fields (pid/name/exe_path/package_full_name) come from the
    /// leader; CPU/memory/disk/net are summed across every member.
    leader: ProcessRow,
    /// All members, including the leader at `[0]`.
    members: Vec<ProcessRow>,
}

/// Groups `rows` (already sorted by the active column) by process name.
/// Group order follows each group's first-seen position in `rows`, not a
/// separate alphabetical pass - the old Slint app's `ProcessTreeBuilder`
/// always regrouped alphabetically regardless of the active sort, which
/// would fight the table's own per-column sorting here.
///
/// `leader_pid`, if it names a member of a group, is promoted to that
/// group's leader instead of the default lowest-pid choice - otherwise
/// selecting (or pinning) any instance but the lowest-pid one would vanish
/// from view the instant its group collapses in the UI, since only the
/// leader row renders at all when collapsed.
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
            // Base order is lowest-pid-first, stable across snapshots as
            // long as that process stays alive; the requested leader (if a
            // member) then takes the slot on top of that.
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

/// Flattens `groups` into the single sequence the table actually renders:
/// leaders at `depth: 0`, members of any group named in `expanded`
/// following at `depth: 1`. Purely presentation - `depth`/`is_expanded`/
/// `has_children` exist because `guinea::widgets::table` wants a flat
/// `Vec<T>` with per-row chevron/indent flags, not because they mean
/// anything about the process itself.
///
/// This runs fresh every render; `pin_display_row` (applied by the caller
/// afterward) is what actually keeps the selected row from jumping to a
/// new position on every re-sort.
pub(super) fn flatten_for_display(
    groups: &[ProcessGroup],
    expanded: &HashSet<String>,
) -> Vec<DisplayRow> {
    let mut out = Vec::new();
    for group in groups {
        let has_children = group.members.len() > 1;
        let is_expanded = has_children && expanded.contains(&group.leader.name);

        out.push(DisplayRow {
            row: group.leader.clone(),
            depth: 0,
            has_children,
            is_expanded,
            group_size: group.members.len(),
        });

        if is_expanded {
            out.extend(group.members[1..].iter().map(|r| DisplayRow {
                row: r.clone(),
                depth: 1,
                has_children: false,
                is_expanded: false,
                group_size: 1,
            }));
        }
    }
    out
}

/// Caches [`group_by_name`]'s output across renders that don't change what
/// it depends on - `rows` (compared by pointer, not value: `state.rows` is
/// an `Rc<[ProcessRow]>` that's only replaced wholesale on a new `SetRows`,
/// so a stale render triggered by something unrelated, e.g. a metrics tick
/// or a column-width drag, still points at the exact same allocation) and
/// `selected` (which promotes a group member to leader). Grouping clones
/// every row once; without this, that cost was paid on every render
/// regardless of whether either input had actually changed.
pub(super) struct GroupsCache {
    rows_ptr: *const ProcessRow,
    rows_len: usize,
    selected: Option<u32>,
    groups: Vec<ProcessGroup>,
}

impl GroupsCache {
    pub(super) fn empty() -> Self {
        Self {
            rows_ptr: std::ptr::null(),
            rows_len: 0,
            selected: None,
            groups: Vec::new(),
        }
    }

    pub(super) fn get(&mut self, rows: &[ProcessRow], selected: Option<u32>) -> &[ProcessGroup] {
        let rows_ptr = rows.as_ptr();
        if self.rows_ptr != rows_ptr || self.rows_len != rows.len() || self.selected != selected {
            self.groups = group_by_name(rows, selected);
            self.rows_ptr = rows_ptr;
            self.rows_len = rows.len();
            self.selected = selected;
        }
        &self.groups
    }
}

#[derive(Clone)]
pub(super) struct DisplayRow {
    pub(super) row: ProcessRow,
    pub(super) depth: u8,
    pub(super) has_children: bool,
    pub(super) is_expanded: bool,
    pub(super) group_size: usize,
}

/// Keeps `target_pid`'s row at its previous on-screen position, the same
/// way `domain2::ProcessesActor::sort_rows_pinned` stabilizes the raw list
/// before grouping - but that pin operates on individual `ProcessRow`s by
/// index, and grouping can collapse several of those into one displayed
/// row, or promote a different member to the leader slot. Once the rows
/// reaching the table are `DisplayRow`s, the *rendered* row's position is
/// the only stability guarantee that still means anything, so it has to be
/// re-applied here on the post-grouping list.
///
/// Returns the row's new position (`None` if `target_pid` isn't rendered
/// at all right now, e.g. a non-leader member of a collapsed group) - feed
/// that back in as `prev_pos` on the next call to keep the pin anchored.
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
        ProcessRow {
            pid,
            name: name.to_string(),
            cpu_percent: pid as f32, // distinct per row, handy for eyeballing failures
            memory_bytes: 0,
            disk_bytes: 0,
            net_bytes: 0,
            exe_path: String::new(),
            package_full_name: String::new(),
        }
    }

    fn pids(rows: &[DisplayRow]) -> Vec<u32> {
        rows.iter().map(|d| d.row.pid).collect()
    }

    fn group(leader: ProcessRow, rest: Vec<ProcessRow>) -> ProcessGroup {
        let mut members = vec![leader.clone()];
        members.extend(rest);
        ProcessGroup { leader, members }
    }

    // --- group_by_name: pure domain computation, no UI concepts ---

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
        // Regression: before leader promotion, selecting/pinning any group
        // member but the lowest-pid one made it vanish the instant its
        // group collapsed in the UI, because only the (unrelated) leader
        // row rendered.
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

        // cpu_percent was seeded as `pid as f32` in the `row` helper.
        assert_eq!(groups[0].leader.cpu_percent, 1.0 + 2.0);
    }

    // --- flatten_for_display: presentation only ---

    #[test]
    fn collapsed_groups_render_as_a_single_leader_row() {
        let groups = vec![group(
            row(2, "chrome.exe"),
            vec![row(5, "chrome.exe"), row(9, "chrome.exe")],
        )];
        let out = flatten_for_display(&groups, &HashSet::new());

        assert_eq!(pids(&out), vec![2]);
        assert!(out[0].has_children);
        assert_eq!(out[0].group_size, 3);
    }

    #[test]
    fn expanding_a_group_inserts_members_right_after_its_leader() {
        let groups = vec![
            group(row(10, "alpha"), vec![]),
            group(row(2, "chrome.exe"), vec![row(5, "chrome.exe")]),
            group(row(20, "zeta"), vec![]),
        ];
        let mut expanded = HashSet::new();
        expanded.insert("chrome.exe".to_string());
        let out = flatten_for_display(&groups, &expanded);

        // Leader (2) then its one member (5), flanked by the untouched
        // neighbors - nothing before/after the group moved.
        assert_eq!(pids(&out), vec![10, 2, 5, 20]);
        assert_eq!(out[1].depth, 0);
        assert_eq!(out[2].depth, 1);
        assert!(!out[2].has_children);
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
            })
            .collect();

        // First call: nothing to anchor to yet, just report where it is.
        let mut first = display.clone();
        let pos = pin_display_row(&mut first, Some(2), None);
        assert_eq!(pos, Some(1));
        assert_eq!(
            pids(&first),
            vec![1, 2, 3],
            "no prior position - nothing should move yet"
        );

        // Simulate a re-sort that pushed pid 2 to the front; the pin
        // should pull it back to its previously-reported index.
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
        }];
        // pid 99 doesn't exist in this list - e.g. a non-leader member of
        // a collapsed group.
        let pos = pin_display_row(&mut display, Some(99), Some(0));
        assert_eq!(pos, None);
    }
}
