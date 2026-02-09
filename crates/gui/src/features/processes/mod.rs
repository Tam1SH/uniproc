use super::Feature;
use crate::core::reactor::Reactor;
use crate::{AppWindow, MachineStats, Navigation, ProcessBridge, ProcessGroup};

use crate::core::actor::addr::Addr;
use crate::core::actor::event_bus::EVENT_BUS;
use crate::features::navigation::utils::get_tab_name_by_index;
use crate::features::navigation::TabChanged;
use crate::features::processes::actors::*;
use crate::features::processes::process_tree::ProcessTreeState;
use crate::features::processes::providers::{IconProvider, NameProvider};
use crate::features::run_task::RunTaskFeature;
use crate::scanner::types::ProcessScanner;
use crate::scanner::windows::WindowsScanner;
use slint::{ComponentHandle, Model, VecModel};
use std::ops::DerefMut;
use std::rc::Rc;
use std::time::Duration;

mod actors;
mod icone_process;
mod process_tree;
mod providers;

pub struct ProcessFeature {
    pub show_icons: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SortField {
    Name,
    Cpu,
    Memory,
    Disk,
    Network,
    None,
}

impl Feature for ProcessFeature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        RunTaskFeature.install(reactor, ui)?;

        let ui_model = Rc::new(VecModel::<ProcessGroup>::default());
        ui.global::<MachineStats>()
            .set_process_groups(ui_model.clone().into());

        let active_index = ui.global::<Navigation>().get_active_tab();
        let current_active_name = get_tab_name_by_index(active_index);

        let state = ProcessActor {
            tree: ProcessTreeState::new(self.show_icons),
            name_provider: NameProvider::new(),
            icon_provider: IconProvider::new(),
            ui_model: ui_model.clone(),
            sort_by: SortField::Cpu,
            sort_descending: true,
            selected_pid: None,
            frozen_index: None,
            last_known_entry: None,
            last_scan_result: None,
            scanner: Some(WindowsScanner::new()),
            is_active: current_active_name == "Processes",
        };

        let addr = Addr::new(state, ui.as_weak());
        let bridge = ui.global::<ProcessBridge>();

        bridge.on_sort_by(addr.handler_with(Sort));
        bridge.on_toggle_expand_group(addr.handler_with(ToggleExpand));
        bridge.on_terminate(addr.handler(TerminateSelected));

        let addr_for_sub = addr.clone();

        EVENT_BUS.with(|bus| {
            bus.subscribe::<ProcessActor, TabChanged, _>(addr_for_sub);
        });

        let a = addr.clone();
        bridge.on_select_process(move |pid, idx| {
            a.send(Select {
                pid: pid as u32,
                idx: idx as usize,
            })
        });

        addr.send(ScanTick);
        let a = addr.clone();
        reactor.add_loop(Duration::from_millis(1500), move || a.send(ScanTick));

        Ok(())
    }
}
