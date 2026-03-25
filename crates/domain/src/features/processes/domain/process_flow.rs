use crate::features::processes::domain::process_tree::{sort_processes_inplace, ProcessTreeState};
use crate::features::processes::services::metadata::ProcessMetadata;
use crate::features::processes::ui::bridge::BridgeSnapshot;
use app_contracts::features::processes::{FieldDefDto, ProcessEntryVm, ProcessGroupVm};

#[derive(Clone, Debug)]
pub struct SortState {
    pub metric_field_id: Option<String>,
    pub metric_descending: bool,
    pub name_descending: Option<bool>,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            metric_field_id: Some("cpu".to_string()),
            metric_descending: true,
            name_descending: None,
        }
    }
}

pub struct ProcessFlowState {
    tree: ProcessTreeState,
    selected_pid: Option<u32>,
    frozen_index: Option<usize>,
    last_known_entry: Option<ProcessEntryVm>,
    last_snapshot: Option<BridgeSnapshot>,
    sort: SortState,
}

impl ProcessFlowState {
    pub fn new(show_icons: bool) -> Self {
        Self {
            tree: ProcessTreeState::new(show_icons),
            selected_pid: None,
            frozen_index: None,
            last_known_entry: None,
            last_snapshot: None,
            sort: SortState::default(),
        }
    }

    pub fn set_snapshot(&mut self, snapshot: BridgeSnapshot) {
        self.last_snapshot = Some(snapshot);
    }

    pub fn has_snapshot(&self) -> bool {
        self.last_snapshot.is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selected_pid = None;
        self.frozen_index = None;
    }

    pub fn select(&mut self, pid: u32, idx: usize) {
        self.selected_pid = Some(pid);
        self.frozen_index = Some(idx);
    }

    pub fn selected_name_for_pid(&self, pid: u32) -> Option<String> {
        self.last_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.processes.iter().find(|p| p.pid == pid))
            .map(|p| p.name.clone())
    }

    pub fn toggle_expand(&mut self, group_id: String) {
        self.tree.toggle_expand(group_id);
    }

    pub fn toggle_sort(&mut self, field: &str) -> SortState {
        if field == "name" {
            self.toggle_name_sort();
        } else {
            self.toggle_metric_sort(field);
        }

        self.sort.clone()
    }

    pub fn build_groups(
        &mut self,
        metadata: &mut dyn ProcessMetadata,
    ) -> Option<Vec<ProcessGroupVm>> {
        let snapshot = self.last_snapshot.as_ref()?;

        let mut groups = self.tree.build_ui_groups(&snapshot.processes, metadata);
        sort_processes_inplace(
            &mut groups,
            self.sort.metric_field_id.as_deref(),
            self.sort.metric_descending,
            self.sort.name_descending,
        );
        self.apply_group_selection_logic(&mut groups);

        Some(groups)
    }

    pub fn column_defs(&self) -> Vec<FieldDefDto> {
        self.last_snapshot
            .as_ref()
            .map(|s| s.column_defs.clone())
            .unwrap_or_default()
    }

    fn toggle_name_sort(&mut self) {
        self.sort.name_descending = match self.sort.name_descending {
            None => Some(false),
            Some(false) => Some(true),
            Some(true) => None,
        };
    }

    fn toggle_metric_sort(&mut self, metric_id: &str) {
        if self.sort.metric_field_id.as_deref() == Some(metric_id) {
            self.sort.metric_descending = !self.sort.metric_descending;
        } else {
            self.sort.metric_field_id = Some(metric_id.to_string());
            self.sort.metric_descending = true;
        }
    }

    fn apply_group_selection_logic(&mut self, groups: &mut Vec<ProcessGroupVm>) {
        let Some(sel_pid) = self.selected_pid else {
            return;
        };

        let current_pos = groups.iter().position(|g| {
            g.parent.pid == sel_pid as i32 || g.children.iter().any(|c| c.pid == sel_pid as i32)
        });

        if let Some(pos) = current_pos {
            self.last_known_entry = Some(groups[pos].parent.clone());

            if let Some(target_idx) = self.frozen_index {
                let target_idx = target_idx.min(groups.len().saturating_sub(1));
                if pos != target_idx {
                    let selected_group = groups.remove(pos);
                    groups.insert(target_idx, selected_group);
                }
            }
        } else if let Some(ghost) = &self.last_known_entry {
            let mut terminated = ghost.clone();
            terminated.is_dead = true;

            let ghost_group = ProcessGroupVm {
                parent: terminated,
                children: Vec::new(),
            };

            let idx = self.frozen_index.unwrap_or(0).min(groups.len());
            groups.insert(idx, ghost_group);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProcessFlowState;

    #[test]
    fn toggle_sort_uses_raw_metric_id_without_remap() {
        let mut flow = ProcessFlowState::new(false);
        let state = flow.toggle_sort("disk");
        assert_eq!(state.metric_field_id.as_deref(), Some("disk"));
        assert!(state.metric_descending);
    }

    #[test]
    fn toggle_name_sort_cycles_none_asc_desc_none() {
        let mut flow = ProcessFlowState::new(false);

        let s1 = flow.toggle_sort("name");
        assert_eq!(s1.name_descending, Some(false));

        let s2 = flow.toggle_sort("name");
        assert_eq!(s2.name_descending, Some(true));

        let s3 = flow.toggle_sort("name");
        assert_eq!(s3.name_descending, None);
    }
}
