use crate::processes_impl::services::metadata::ProcessMetadataService;
use app_contracts::features::processes::{ProcessEntryVm, ProcessFieldDto, ProcessNodeDto};
use app_table::flow::{TableDataBuilder, TableNode};
use foundation_services::caches::icons::IconRequest;
use slint::SharedString;
use std::collections::HashSet;

pub struct ProcessTreeBuilder<'a> {
    pub metadata: &'a ProcessMetadataService,
    pub grouping_scratchpad: &'a mut Vec<(SharedString, usize)>,
}

impl<'a> TableDataBuilder<ProcessNodeDto, ProcessEntryVm> for ProcessTreeBuilder<'a> {
    fn build_tree(
        &mut self,
        items: &[ProcessNodeDto],
        expanded: &HashSet<SharedString>,
        out: &mut Vec<TableNode<ProcessEntryVm>>,
    ) {
        let buffer: &mut Vec<_> = self.grouping_scratchpad;
        buffer.clear();

        for (idx, proc) in items.iter().enumerate() {
            let clean_name = self.metadata.clean_name(&proc.name);
            buffer.push((clean_name, idx));
        }

        buffer.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut i = 0;

        while i < buffer.len() {
            let group_name = buffer[i].0.clone();
            let mut j = i;
            while j < buffer.len() && buffer[j].0 == group_name {
                j += 1;
            }

            let group_procs = &buffer[i..j];
            let leader_idx = group_procs
                .iter()
                .map(|(_, idx)| *idx)
                .min_by_key(|idx| items[*idx].pid)
                .unwrap();

            let has_children = group_procs.len() > 1;
            let is_expanded = expanded.contains(&group_name);

            let mut parent_vm = to_vm(self.metadata, &items[leader_idx], 0);
            parent_vm.name = group_name.clone();
            parent_vm.has_children = has_children;
            parent_vm.is_expanded = is_expanded;

            let mut children = Vec::new();
            if is_expanded && has_children {
                for &(_, idx) in group_procs {
                    if idx != leader_idx {
                        children.push(TableNode {
                            vm: to_vm(self.metadata, &items[idx], 1),
                            group_id: None,
                            has_children: false,
                            is_expanded: false,
                            level: 1,
                            children: Vec::new(),
                        });
                    }
                }
            }

            out.push(TableNode {
                vm: parent_vm,
                group_id: Some(group_name.clone()),
                has_children,
                is_expanded,
                level: 0,
                children,
            });
            i = j;
        }
    }
}

fn to_vm(metadata: &ProcessMetadataService, proc: &ProcessNodeDto, depth: i32) -> ProcessEntryVm {
    ProcessEntryVm {
        pid: proc.pid as i32,
        name: metadata.clean_name(&proc.name),
        icon: metadata.icon_by_path(
            IconRequest::builder()
                .path(proc.exe_path.as_str())
                .maybe_package_full_name(proc.package_name.as_ref().map(|n| n.as_str()))
                .build(),
        ),
        depth,
        has_children: false,
        is_expanded: false,
        is_dead: false,
        fields: proc
            .fields
            .iter()
            .map(|f| ProcessFieldDto {
                id: f.id.clone(),
                text: f.text.clone(),
                numeric: f.numeric,
                threshold: f.threshold,
            })
            .collect(),
    }
}
