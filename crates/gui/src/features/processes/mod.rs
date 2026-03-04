use super::Feature;
use crate::core::reactor::Reactor;
use crate::{AppWindow, Navigation, ProcessGroup, ProcessesFeatureGlobal};

use crate::core::actor::addr::Addr;
use crate::core::actor::event_bus::EVENT_BUS;
use crate::features::navigation::utils::get_tab_name_by_index;
use crate::features::navigation::TabChanged;
use crate::features::processes::application::actors::*;
use crate::features::processes::domain::process_flow::ProcessFlowState;
use crate::features::processes::services::metadata::ProcessMetadataService;
use crate::features::run_task::RunTaskFeature;
use scanner::windows::WindowsScanner;
use slint::{ComponentHandle, VecModel};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

mod application;
mod domain;
mod scanner;
mod services;
pub mod ui;

pub struct ProcessFeature {
    pub show_icons: bool,
}

impl Feature for ProcessFeature {
    fn install(self, reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        RunTaskFeature.install(reactor, ui)?;

        let ui_model = Rc::new(VecModel::<ProcessGroup>::default());
        ui.global::<ProcessesFeatureGlobal>()
            .set_process_groups(ui_model.clone().into());

        let active_index = ui.global::<Navigation>().get_active_tab_index();
        let current_active_name = get_tab_name_by_index(active_index);

        let state = ProcessActor {
            flow: ProcessFlowState::new(self.show_icons),
            metadata: ProcessMetadataService::new(),
            ui_model: ui_model.clone(),
            scanners: Some(vec![Box::new(WindowsScanner::new())]),
            widths_by_schema: HashMap::new(),
            is_active: current_active_name == "Processes",
        };

        let addr = Addr::new(state, ui.as_weak());
        let bridge = ui.global::<ProcessesFeatureGlobal>();

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
