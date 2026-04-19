use crate::features::navigation::UiNavigationAdapter;
use app_contracts::features::navigation::{
    AvailableContextDescriptor, TabDescriptor, UiNavigationPort,
};
use context::icons::Icons;
use context::page_status::{PageId, PageStatus, TabId};
use macros::slint_port_adapter;
use slint::private_unstable_api::re_exports::Coord;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

fn update_row<T: Clone + 'static>(
    model: ModelRc<T>,
    id: i32,
    id_getter: impl Fn(&T) -> i32,
    updater: impl FnOnce(&mut T),
) {
    if let Some(idx) = model.iter().position(|item| id_getter(&item) == id) {
        if let Some(mut item) = model.row_data(idx) {
            updater(&mut item);
            model.set_row_data(idx, item);
        }
    }
}

#[slint_port_adapter(window = AppWindow)]
impl UiNavigationPort for UiNavigationAdapter {
    fn set_navigation_tree(&self, ui: &AppWindow, tabs: Vec<TabDescriptor>) {
        let slint_tabs: Vec<_> = tabs
            .into_iter()
            .map(|tab| {
                let pages: Vec<_> = tab
                    .pages
                    .into_iter()
                    .map(|p| crate::PageData {
                        id: p.id.0 as i32,
                        text: p.text.into(),
                        icon: Icons::get(&p.icon_key),
                        status: p.status.into(),
                        error_msg: p.error_msg.into(),
                    })
                    .collect();

                crate::TabData {
                    id: tab.id.0 as i32,
                    title: tab.title.into(),
                    icon: Icons::get(&tab.icon_key),
                    pages: ModelRc::new(VecModel::from(pages)),
                    status: tab.status.into(),
                    error_msg: tab.error_msg.into(),
                    is_closable: tab.is_closable,
                }
            })
            .collect();

        ui.global::<crate::Navigation>()
            .set_tabs(ModelRc::new(VecModel::from(slint_tabs)));
    }

    fn set_available_contexts(&self, ui: &AppWindow, contexts: Vec<AvailableContextDescriptor>) {
        let slint_contexts: Vec<_> = contexts
            .into_iter()
            .map(|context| crate::AvailableContextData {
                context_key: context.context_key.0.to_string().into(),
                title: context.title.into(),
                icon: Icons::get(&context.icon_key),
                status: context.status.into(),
            })
            .collect();

        ui.global::<crate::Navigation>()
            .set_available_contexts(ModelRc::new(VecModel::from(slint_contexts)));
    }

    fn set_active_tab(&self, ui: &AppWindow, tab_id: TabId) {
        let nav = ui.global::<crate::Navigation>();
        if let Some(idx) = nav.get_tabs().iter().position(|t| t.id == tab_id.0 as i32) {
            nav.set_active_tab_index(idx as i32);
        }
    }

    fn set_active_page(&self, ui: &AppWindow, tab_id: TabId, page_id: PageId) {
        let nav = ui.global::<crate::Navigation>();
        if let Some(tab) = nav.get_tabs().iter().find(|t| t.id == tab_id.0 as i32) {
            if let Some(idx) = tab.pages.iter().position(|p| p.id == page_id.0 as i32) {
                nav.set_active_page_index(idx as i32);
            }
        }
    }

    fn set_page_status(&self, ui: &AppWindow, tab_id: TabId, page_id: PageId, status: PageStatus) {
        if let Some(tab) = ui
            .global::<crate::Navigation>()
            .get_tabs()
            .iter()
            .find(|t| t.id == tab_id.0 as i32)
        {
            update_row(
                tab.pages,
                page_id.0 as i32,
                |p| p.id,
                |p| p.status = status.into(),
            );
        }
    }

    fn set_page_error(&self, ui: &AppWindow, tab_id: TabId, page_id: PageId, msg: String) {
        if let Some(tab) = ui
            .global::<crate::Navigation>()
            .get_tabs()
            .iter()
            .find(|t| t.id == tab_id.0 as i32)
        {
            update_row(
                tab.pages,
                page_id.0 as i32,
                |p| p.id,
                |p| p.error_msg = msg.into(),
            );
        }
    }

    fn set_tab_status(&self, ui: &AppWindow, tab_id: TabId, status: PageStatus) {
        update_row(
            ui.global::<crate::Navigation>().get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| t.status = status.into(),
        );
    }

    fn set_tab_error(&self, ui: &AppWindow, tab_id: TabId, msg: String) {
        update_row(
            ui.global::<crate::Navigation>().get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| t.error_msg = msg.into(),
        );
    }

    fn set_switch_transition(&self, ui: &AppWindow, from_index: i32, to_index: i32, progress: f32) {
        let nav = ui.global::<crate::Navigation>();
        nav.set_switch_from_index(from_index);
        nav.set_switch_to_index(to_index);
        nav.set_switch_progress(progress);
    }

    fn set_side_bar_width(&self, ui: &AppWindow, width: u64) {
        ui.global::<crate::Navigation>()
            .set_side_bar_width(width as Coord)
    }
}

impl From<PageStatus> for crate::PageStatus {
    fn from(status: PageStatus) -> Self {
        match status {
            PageStatus::Loading => crate::PageStatus::Loading,
            PageStatus::Ready => crate::PageStatus::Ready,
            PageStatus::Error => crate::PageStatus::Error,
            PageStatus::Inactive => crate::PageStatus::Inactive,
        }
    }
}
