use crate::{AppWindow, Navigation, PageData, TabData};
use app_contracts::features::navigation::{NavigationUiBindings, NavigationUiPort, TabDescriptor};
use app_core::icons::Icons;
use context::page_status::{PageId, PageStatus, TabId};
use macros::ui_adapter;
use slint::private_unstable_api::re_exports::Coord;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

#[derive(Clone)]
pub struct NavigationUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl NavigationUiAdapter {
    pub fn new(ui: slint::Weak<AppWindow>) -> Self {
        Self { ui }
    }
}

#[ui_adapter]
impl NavigationUiPort for NavigationUiAdapter {
    fn set_navigation_tree(&self, ui: &AppWindow, tabs: Vec<TabDescriptor>) {
        let slint_tabs: Vec<_> = tabs
            .into_iter()
            .map(|tab| {
                let pages: Vec<_> = tab
                    .pages
                    .into_iter()
                    .map(|p| PageData {
                        id: p.id.0 as i32,
                        text: p.text.into(),
                        icon: Icons::get(&p.icon_key),
                        status: p.status.into(),
                        error_msg: p.error_msg.into(),
                    })
                    .collect();

                TabData {
                    id: tab.id.0 as i32,
                    title: tab.title.into(),
                    pages: ModelRc::new(VecModel::from(pages)),
                    status: tab.status.into(),
                    error_msg: tab.error_msg.into(),
                }
            })
            .collect();

        ui.global::<Navigation>()
            .set_tabs(ModelRc::new(VecModel::from(slint_tabs)));
    }

    fn set_active_tab(&self, ui: &AppWindow, tab_id: TabId) {
        let nav = ui.global::<Navigation>();
        if let Some(idx) = nav.get_tabs().iter().position(|t| t.id == tab_id.0 as i32) {
            nav.set_active_tab_index(idx as i32);
        }
    }

    fn set_active_page(&self, ui: &AppWindow, tab_id: TabId, page_id: PageId) {
        let nav = ui.global::<Navigation>();
        if let Some(tab) = nav.get_tabs().iter().find(|t| t.id == tab_id.0 as i32) {
            if let Some(idx) = tab.pages.iter().position(|p| p.id == page_id.0 as i32) {
                nav.set_active_page_index(idx as i32);
            }
        }
    }

    fn set_page_status(&self, ui: &AppWindow, tab_id: TabId, page_id: PageId, status: PageStatus) {
        if let Some(tab) = ui
            .global::<Navigation>()
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
            .global::<Navigation>()
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
            ui.global::<Navigation>().get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| t.status = status.into(),
        );
    }

    fn set_tab_error(&self, ui: &AppWindow, tab_id: TabId, msg: String) {
        update_row(
            ui.global::<Navigation>().get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| t.error_msg = msg.into(),
        );
    }

    fn set_switch_transition(&self, ui: &AppWindow, from_idx: i32, to_idx: i32, progress: f32) {
        let nav = ui.global::<Navigation>();
        nav.set_switch_from_index(from_idx);
        nav.set_switch_to_index(to_idx);
        nav.set_switch_progress(progress);
    }

    fn set_switch_progress(&self, ui: &AppWindow, progress: f32) {
        ui.global::<Navigation>().set_switch_progress(progress)
    }

    fn set_content_visible(&self, ui: &AppWindow, visible: bool) {
        ui.global::<Navigation>().set_content_visible(visible)
    }

    fn set_side_bar_width(&self, ui: &AppWindow, width: u64) {
        ui.global::<Navigation>().set_side_bar_width(width as Coord)
    }
}

#[ui_adapter]
impl NavigationUiBindings for NavigationUiAdapter {
    #[ui_action(scope = "ui.navigation.page_switch", target = "tab_id,page_id")]
    fn on_request_page_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId, PageId) + 'static,
    {
        ui.global::<Navigation>()
            .on_request_page_switch(move |t, p| handler(TabId(t as u32), PageId(p as u32)));
    }

    #[ui_action(scope = "ui.navigation.sidebar_width", target = "width")]
    fn on_side_bar_width_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(u64) + 'static,
    {
        ui.global::<Navigation>()
            .on_side_bar_width_changed(move |w| handler(w as u64));
    }

    #[ui_action(scope = "ui.navigation.tab_switch", target = "tab_id")]
    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_h = self.ui.clone();
        ui.global::<Navigation>().on_request_tab_switch(move |idx| {
            if let Some(tab) = ui_h
                .upgrade()
                .and_then(|ui| ui.global::<Navigation>().get_tabs().row_data(idx as usize))
            {
                handler(TabId(tab.id as u32));
            }
        });
    }

    #[ui_action(scope = "ui.navigation.tab_close", target = "tab_id")]
    fn on_request_tab_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_h = self.ui.clone();
        ui.global::<Navigation>().on_request_tab_close(move |idx| {
            if let Some(tab) = ui_h
                .upgrade()
                .and_then(|ui| ui.global::<Navigation>().get_tabs().row_data(idx as usize))
            {
                handler(TabId(tab.id as u32));
            }
        });
    }

    #[ui_action(scope = "ui.navigation.tab_add")]
    fn on_request_tab_add<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<Navigation>().on_request_tab_add(handler);
    }
}

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
