use crate::features::processes::services::metadata::ProcessMetadata;
use crate::features::processes::ui::slint_bridge::SlintProcess;
use crate::{ProcessEntry, ProcessField, ProcessGroup};
use slint::{Image, Model, ModelRc, SharedString, VecModel};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct ProcessTreeState {
    expanded_groups: HashSet<SharedString>,
    show_icons: bool,
}

impl ProcessTreeState {
    pub fn new(show_icons: bool) -> Self {
        Self {
            expanded_groups: HashSet::new(),
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
        processes: &[SlintProcess],
        metadata: &mut dyn ProcessMetadata,
    ) -> Vec<ProcessGroup> {
        let mut groups_map: HashMap<SharedString, Vec<&SlintProcess>> = HashMap::new();

        for proc in processes {
            let clean_name = metadata.clean_name(&proc.name);
            groups_map.entry(clean_name).or_default().push(proc);
        }

        let mut result = Vec::new();

        for (group_name, mut procs) in groups_map {
            procs.sort_unstable_by_key(|p| p.pid);

            let leader = procs[0];
            let is_expanded = self.expanded_groups.contains(&group_name);
            let has_children = procs.len() > 1;

            let mut parent = self.to_entry(leader, 0, metadata);
            parent.name = group_name.clone();
            parent.has_children = has_children;
            parent.is_expanded = is_expanded;

            let children = if is_expanded && has_children {
                procs
                    .iter()
                    .skip(1)
                    .map(|p| self.to_entry(p, 1, metadata))
                    .collect()
            } else {
                vec![]
            };

            result.push(ProcessGroup {
                parent,
                children: ModelRc::new(VecModel::from(children)),
            });
        }

        result
    }

    fn to_entry(
        &self,
        proc: &SlintProcess,
        depth: i32,
        metadata: &mut dyn ProcessMetadata,
    ) -> ProcessEntry {
        let fields: Vec<ProcessField> = proc
            .fields
            .iter()
            .map(|f| ProcessField {
                id: f.id.clone(),
                text: f.text.clone(),
                width_px: f.width_px,
                numeric: f.numeric,
                threshold: f.threshold,
            })
            .collect();

        ProcessEntry {
            pid: proc.pid as i32,
            name: metadata.clean_name(&proc.name),
            icon: if self.show_icons
                && let Some(path) = &proc.exe_path
            {
                metadata.icon_by_path(path)
            } else {
                Image::default()
            },
            depth,
            has_children: false,
            is_expanded: false,
            is_dead: false,
            fields: Rc::new(VecModel::from(fields)).into(),
        }
    }
}

pub fn sort_processes_inplace(
    groups: &mut Vec<ProcessGroup>,
    metric_field_id: Option<&str>,
    metric_descending: bool,
    name_descending: Option<bool>,
) {
    let field_numeric = |entry: &ProcessEntry, id: &str| -> f32 {
        entry
            .fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.numeric)
            .unwrap_or(-1.0)
    };

    groups.sort_by(|a, b| {
        if let Some(metric_id) = metric_field_id {
            let metric_cmp = field_numeric(&a.parent, metric_id)
                .partial_cmp(&field_numeric(&b.parent, metric_id))
                .unwrap_or(Ordering::Equal);
            if metric_cmp != Ordering::Equal {
                return if metric_descending {
                    metric_cmp.reverse()
                } else {
                    metric_cmp
                };
            }
        }

        if let Some(is_desc) = name_descending {
            let name_cmp = a.parent.name.cmp(&b.parent.name);
            if name_cmp != Ordering::Equal {
                return if is_desc {
                    name_cmp.reverse()
                } else {
                    name_cmp
                };
            }
        }

        a.parent.pid.cmp(&b.parent.pid)
    });
}

#[cfg(test)]
mod tests {
    use super::sort_processes_inplace;
    use crate::{ProcessEntry, ProcessField, ProcessGroup};
    use slint::{Image, ModelRc, SharedString, VecModel};
    use std::rc::Rc;

    fn mk_group(name: &str, pid: i32, metric_id: &str, metric_value: f32) -> ProcessGroup {
        let fields = vec![ProcessField {
            id: metric_id.into(),
            text: SharedString::default(),
            width_px: 70,
            numeric: metric_value,
            threshold: 0.0,
        }];

        ProcessGroup {
            parent: ProcessEntry {
                pid,
                name: name.into(),
                icon: Image::default(),
                depth: 0,
                has_children: false,
                is_expanded: false,
                is_dead: false,
                fields: Rc::new(VecModel::from(fields)).into(),
            },
            children: ModelRc::default(),
        }
    }

    #[test]
    fn sort_by_metric_then_name() {
        let mut groups = vec![
            mk_group("beta", 2, "cpu", 10.0),
            mk_group("alpha", 1, "cpu", 10.0),
            mk_group("charlie", 3, "cpu", 20.0),
        ];

        sort_processes_inplace(&mut groups, Some("cpu"), true, Some(false));

        let ordered = groups
            .iter()
            .map(|g| g.parent.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec!["charlie", "alpha", "beta"]);
    }

    #[test]
    fn sort_by_name_only_desc() {
        let mut groups = vec![
            mk_group("alpha", 1, "cpu", 1.0),
            mk_group("charlie", 3, "cpu", 1.0),
            mk_group("beta", 2, "cpu", 1.0),
        ];

        sort_processes_inplace(&mut groups, None, true, Some(true));

        let ordered = groups
            .iter()
            .map(|g| g.parent.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(ordered, vec!["charlie", "beta", "alpha"]);
    }
}
