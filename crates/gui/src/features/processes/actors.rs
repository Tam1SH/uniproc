use crate::core::actor::traits::{Context, Handler, Message, NoOp};
use crate::features::navigation::TabChanged;
use crate::features::processes::process_tree::{sort_processes_inplace, ProcessTreeState};
use crate::features::processes::providers::{IconProvider, NameProvider};
use crate::features::processes::SortField;
use crate::scanner::types::{ProcessScanner, ScanResult};
use crate::scanner::windows::WindowsScanner;
use crate::{
    messages, AppWindow, MachineStats, MainBodyState, ProcessBridge, ProcessEntry, ProcessGroup,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use sysinfo::{Pid, ProcessesToUpdate, System};

pub struct ScanResponse(ScanResult, WindowsScanner);
impl Message for ScanResponse {}

messages! {
    ScanTick,

    Sort(SharedString),
    ToggleExpand(SharedString),
    Select { pid: u32, idx: usize },
    TerminateSelected,
}

pub struct ProcessActor {
    pub tree: ProcessTreeState,
    pub name_provider: NameProvider,
    pub icon_provider: IconProvider,
    pub ui_model: Rc<VecModel<ProcessGroup>>,

    pub sort_by: SortField,
    pub sort_descending: bool,

    pub selected_pid: Option<u32>,
    pub frozen_index: Option<usize>,

    pub last_known_entry: Option<ProcessEntry>,
    pub last_scan_result: Option<ScanResult>,

    pub scanner: Option<WindowsScanner>,

    pub is_active: bool,
}

impl ProcessActor {
    fn stabilize(val: f32) -> f32 {
        (val * 10.0).round() / 10.0
    }

    fn refresh_ui_model(&mut self, ui: &AppWindow) {
        let Some(res) = &self.last_scan_result else {
            return;
        };

        let stats_ui = ui.global::<MachineStats>();
        let update_stat = |cur: f32, new: f32, setter: &dyn Fn(f32)| {
            if (cur - new).abs() > 0.5 {
                setter(new);
            }
        };

        update_stat(stats_ui.get_cpu_usage(), res.stats.cpu_percent, &|v| {
            stats_ui.set_cpu_usage(v)
        });
        update_stat(stats_ui.get_ram_usage(), res.stats.ram_percent, &|v| {
            stats_ui.set_ram_usage(v)
        });
        update_stat(stats_ui.get_disk_usage(), res.stats.disk_percent, &|v| {
            stats_ui.set_disk_usage(v)
        });
        update_stat(stats_ui.get_net_usage(), res.stats.net_percent, &|v| {
            stats_ui.set_net_usage(v)
        });

        self.tree.all_processes = res.processes.iter().map(|p| (p.pid, p.clone())).collect();

        let mut groups =
            self.tree
                .build_ui_groups(&mut self.name_provider, &mut self.icon_provider, &res.stats);

        sort_processes_inplace(&mut groups, self.sort_by, self.sort_descending);
        self.apply_group_selection_logic(&mut groups);

        let target_count = groups.len().min(100);
        while self.ui_model.row_count() > target_count {
            self.ui_model.remove(self.ui_model.row_count() - 1);
        }

        for (i, mut group) in groups.into_iter().take(target_count).enumerate() {
            group.parent.cpu_usage = Self::stabilize(group.parent.cpu_usage);
            group.parent.ram_usage = Self::stabilize(group.parent.ram_usage);
            group.parent.disk_usage = Self::stabilize(group.parent.disk_usage);
            group.parent.net_usage = Self::stabilize(group.parent.net_usage);

            if i < self.ui_model.row_count() {
                if let Some(existing) = self.ui_model.row_data(i) {
                    if existing != group {
                        self.ui_model.set_row_data(i, group);
                    }
                }
            } else {
                self.ui_model.push(group);
            }
        }
    }

    fn apply_group_selection_logic(&mut self, groups: &mut Vec<ProcessGroup>) {
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

            let ghost_group = ProcessGroup {
                parent: terminated,
                children: ModelRc::default(),
            };

            let idx = self.frozen_index.unwrap_or(0).min(groups.len());
            groups.insert(idx, ghost_group);
        }
    }
}

impl Handler<TabChanged, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: TabChanged, ctx: &Context<Self, AppWindow>) {
        self.is_active = msg.name == "Processes";
        if self.is_active {
            ctx.addr().send(ScanTick);
        }
    }
}

impl Handler<ScanTick, AppWindow> for ProcessActor {
    fn handle(&mut self, _msg: ScanTick, ctx: &Context<Self, AppWindow>) {
        if !self.is_active {
            return;
        }

        if let Some(mut scanner) = self.scanner.take() {
            ctx.spawn_task(
                async move {
                    let res = scanner.scan();
                    ScanResponse(res, scanner)
                },
                move |_, _| {},
            );
        }
    }
}

impl Handler<ScanResponse, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: ScanResponse, ctx: &Context<Self, AppWindow>) {
        self.last_scan_result = Some(msg.0);
        self.scanner = Some(msg.1);
        ctx.with_ui(|ui| {
            ui.global::<MainBodyState>()
                .set_is_loading(self.last_scan_result.is_none());
            self.refresh_ui_model(ui);
        });
    }
}

impl Handler<Sort, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: Sort, ctx: &Context<Self, AppWindow>) {
        let field = match msg.0.as_str() {
            "cpu" => SortField::Cpu,
            "memory" => SortField::Memory,
            "disk" => SortField::Disk,
            "network" => SortField::Network,
            "name" => SortField::Name,
            _ => SortField::None,
        };

        if self.sort_by == field {
            self.sort_descending = !self.sort_descending;
        } else {
            self.sort_by = field;
            self.sort_descending = true;
        }

        ctx.with_ui(|ui| {
            let bridge = ui.global::<ProcessBridge>();
            bridge.set_current_sort(msg.0.into());
            bridge.set_current_sort_descending(self.sort_descending);
            self.refresh_ui_model(ui);
        });
    }
}

impl Handler<TerminateSelected, AppWindow> for ProcessActor {
    fn handle(&mut self, _msg: TerminateSelected, ctx: &Context<Self, AppWindow>) {
        let pid = ctx.with_ui(|ui| ui.global::<ProcessBridge>().get_selected_pid());
        let Some(pid) = pid.filter(|&p| p != -1).map(|p| p as u32) else {
            return;
        };

        ctx.spawn_bg(async move {
            let mut system = System::new();
            system.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), false);
            if let Some(process) = system.process(Pid::from_u32(pid)) {
                process.kill();
            }
            NoOp
        });

        self.selected_pid = None;
        self.frozen_index = None;
    }
}

impl Handler<Select, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: Select, ctx: &Context<Self, AppWindow>) {
        self.selected_pid = Some(msg.pid);
        self.frozen_index = Some(msg.idx);
        ctx.with_ui(|ui| {
            let bridge = ui.global::<ProcessBridge>();

            bridge.set_selected_pid(msg.pid as i32);

            if let Some(ScanResult {
                processes,
                stats: _,
            }) = &self.last_scan_result
            {
                if let Some(proc) = processes.iter().find(|p| p.pid == msg.pid) {
                    bridge.set_selected_name(proc.name.clone().into());
                }
            }
        });
    }
}

impl Handler<ToggleExpand, AppWindow> for ProcessActor {
    fn handle(&mut self, msg: ToggleExpand, ctx: &Context<Self, AppWindow>) {
        self.tree.toggle_expand(msg.0);
        ctx.with_ui(|ui| self.refresh_ui_model(ui));
    }
}
