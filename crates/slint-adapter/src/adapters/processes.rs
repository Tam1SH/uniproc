use crate::{
    AppWindow, MainBodyState, ProcessEntry, ProcessField, ProcessGroup, ProcessesFeatureGlobal,
};
use app_contracts::features::processes::{
    FieldDefDto, ProcessEntryVm, ProcessFieldDto, ProcessGroupVm, ProcessesUiBindings,
    ProcessesUiPort,
};
use app_core::app::FromUiWeak;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::rc::Rc;

#[derive(Clone)]
pub struct ProcessesUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl ProcessesUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }

    fn with_ui<F>(&self, f: F)
    where
        F: FnOnce(&AppWindow),
    {
        if let Some(ui) = self.ui.upgrade() {
            f(&ui);
        }
    }
}

impl FromUiWeak<AppWindow> for ProcessesUiAdapter {
    fn from_ui_weak(ui: slint::Weak<AppWindow>) -> Self {
        Self::new(ui)
    }
}

fn to_field_vm(field: ProcessFieldDto) -> ProcessField {
    ProcessField {
        id: field.id.into(),
        text: field.text.into(),
        width_px: field.width_px,
        numeric: field.numeric,
        threshold: field.threshold,
    }
}

fn to_entry_vm(entry: ProcessEntryVm) -> ProcessEntry {
    ProcessEntry {
        pid: entry.pid,
        name: entry.name.into(),
        icon: entry.icon,
        depth: entry.depth,
        has_children: entry.has_children,
        is_expanded: entry.is_expanded,
        is_dead: entry.is_dead,
        fields: ModelRc::new(VecModel::from(
            entry
                .fields
                .into_iter()
                .map(to_field_vm)
                .collect::<Vec<_>>(),
        )),
    }
}

impl ProcessesUiPort for ProcessesUiAdapter {
    fn set_process_groups(&self, groups: Vec<ProcessGroupVm>) {
        self.with_ui(|ui| {
            let groups = groups
                .into_iter()
                .map(|g| ProcessGroup {
                    parent: to_entry_vm(g.parent),
                    children: ModelRc::new(VecModel::from(
                        g.children.into_iter().map(to_entry_vm).collect::<Vec<_>>(),
                    )),
                })
                .collect::<Vec<_>>();
            ui.global::<ProcessesFeatureGlobal>()
                .set_process_groups(ModelRc::new(VecModel::from(groups)));
        });
    }

    fn set_column_defs(&self, defs: Vec<FieldDefDto>) {
        self.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            let defs = defs
                .into_iter()
                .map(crate::FieldDef::from)
                .collect::<Vec<_>>();
            bridge.set_column_defs(Rc::new(VecModel::from(defs)).into());
        });
    }

    fn set_loading(&self, loading: bool) {
        self.with_ui(|ui| ui.global::<MainBodyState>().set_is_loading(loading));
    }

    fn get_selected_pid(&self) -> i32 {
        if let Some(ui) = self.ui.upgrade() {
            return ui.global::<ProcessesFeatureGlobal>().get_selected_pid();
        }
        -1
    }

    fn set_selected_pid(&self, pid: i32) {
        self.with_ui(|ui| ui.global::<ProcessesFeatureGlobal>().set_selected_pid(pid));
    }

    fn set_selected_name(&self, name: String) {
        self.with_ui(|ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .set_selected_name(name.into())
        });
    }

    fn set_sort_state(&self, field: String, descending: bool) {
        self.with_ui(|ui| {
            let bridge = ui.global::<ProcessesFeatureGlobal>();
            bridge.set_current_sort(field.into());
            bridge.set_current_sort_descending(descending);
        });
    }
}

impl ProcessesUiBindings for ProcessesUiAdapter {
    fn on_sort_by<F>(&self, handler: F)
    where
        F: Fn(String) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_sort_by(move |field| handler(field.to_string()));
        });
    }

    fn on_toggle_expand_group<F>(&self, handler: F)
    where
        F: Fn(String) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_toggle_expand_group(move |group| handler(group.to_string()));
        });
    }

    fn on_terminate<F>(&self, handler: F)
    where
        F: Fn() + 'static,
    {
        self.with_ui(move |ui| ui.global::<ProcessesFeatureGlobal>().on_terminate(handler));
    }

    fn on_select_process<F>(&self, handler: F)
    where
        F: Fn(i32, i32) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<ProcessesFeatureGlobal>()
                .on_select_process(move |pid, idx| handler(pid, idx));
        });
    }
}
