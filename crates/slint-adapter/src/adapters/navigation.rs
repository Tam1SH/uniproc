use crate::{AppWindow, Navigation};
use app_contracts::features::navigation::{NavigationUiBindings, NavigationUiPort, TabDescriptor};
use app_core::icons::Icons;
use context::page_status::{PageId, PageStatus, TabId};
use macros::ui_adapter;
use slint::private_unstable_api::re_exports::Coord;
use slint::ComponentHandle;
use slint::{Model, ModelRc, VecModel};
use tracing::info;

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
        let nav = ui.global::<Navigation>();

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
                        status: match p.status {
                            PageStatus::Loading => crate::PageStatus::Loading,
                            PageStatus::Ready => crate::PageStatus::Ready,
                            PageStatus::Error => crate::PageStatus::Error,
                            PageStatus::Inactive => crate::PageStatus::Inactive,
                        },
                        error_msg: p.error_msg.into(),
                        icon_key: Default::default(),
                    })
                    .collect();

                crate::TabData {
                    id: tab.id.0 as i32,
                    title: tab.title.into(),
                    pages: ModelRc::new(VecModel::from(pages)),
                    status: match tab.status {
                        PageStatus::Loading => crate::PageStatus::Loading,
                        PageStatus::Ready => crate::PageStatus::Ready,
                        PageStatus::Error => crate::PageStatus::Error,
                        PageStatus::Inactive => crate::PageStatus::Inactive,
                    },
                    error_msg: tab.error_msg.into(),
                }
            })
            .collect();

        info!("{:?}", &slint_tabs);
        nav.set_tabs(ModelRc::new(VecModel::from(slint_tabs)));
    }

    fn set_active_tab(&self, ui: &AppWindow, tab_id: TabId) {
        let nav = ui.global::<Navigation>();
        let tabs = nav.get_tabs();
        for i in 0..tabs.row_count() {
            if let Some(tab) = tabs.row_data(i) {
                if tab.id == tab_id.0 as i32 {
                    nav.set_active_tab_index(i as i32);
                    return;
                }
            }
        }
    }

    fn set_active_page(&self, ui: &AppWindow, _tab_id: TabId, page_id: PageId) {
        let nav = ui.global::<Navigation>();
        let tab = nav
            .get_tabs()
            .iter()
            .take(1)
            .collect::<Vec<_>>()
            .first()
            .cloned()
            .unwrap();
        let pages = tab.pages;
        for i in 0..pages.row_count() {
            if let Some(page) = pages.row_data(i) {
                if page.id == page_id.0 as i32 {
                    nav.set_active_page_index(i as i32);
                    return;
                }
            }
        }
    }

    fn set_page_status(&self, ui: &AppWindow, _tab_id: TabId, page_id: PageId, status: PageStatus) {
        let ui_status = match status {
            PageStatus::Loading => crate::PageStatus::Loading,
            PageStatus::Ready => crate::PageStatus::Ready,
            PageStatus::Error => crate::PageStatus::Error,
            PageStatus::Inactive => crate::PageStatus::Inactive,
        };

        let nav = ui.global::<Navigation>();
        let tab = nav
            .get_tabs()
            .iter()
            .take(1)
            .collect::<Vec<_>>()
            .first()
            .cloned()
            .unwrap();
        update_row(
            tab.pages,
            page_id.0 as i32,
            |p| p.id,
            |p| {
                p.status = ui_status;
            },
        );
    }

    fn set_page_error(&self, ui: &AppWindow, _tab_id: TabId, page_id: PageId, msg: String) {
        let nav = ui.global::<Navigation>();
        let tab = nav
            .get_tabs()
            .iter()
            .take(1)
            .collect::<Vec<_>>()
            .first()
            .cloned()
            .unwrap();

        update_row(
            tab.pages,
            page_id.0 as i32,
            |p| p.id,
            |p| {
                p.error_msg = msg.clone().into();
            },
        );
    }

    fn set_tab_status(&self, ui: &AppWindow, tab_id: TabId, status: PageStatus) {
        let ui_status = match status {
            PageStatus::Loading => crate::PageStatus::Loading,
            PageStatus::Ready => crate::PageStatus::Ready,
            PageStatus::Error => crate::PageStatus::Error,
            PageStatus::Inactive => crate::PageStatus::Inactive,
        };
        let nav = ui.global::<Navigation>();

        update_row(
            nav.get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| {
                t.status = ui_status;
            },
        );
    }

    fn set_tab_error(&self, ui: &AppWindow, tab_id: TabId, msg: String) {
        update_row(
            ui.global::<Navigation>().get_tabs(),
            tab_id.0 as i32,
            |t| t.id,
            |t| {
                t.error_msg = msg.clone().into();
            },
        );
    }

    fn set_switch_transition(&self, ui: &AppWindow, from_index: i32, to_index: i32, progress: f32) {
        let nav = ui.global::<Navigation>();
        nav.set_switch_from_index(from_index);
        nav.set_switch_to_index(to_index);
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
    fn on_request_page_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId, PageId) + 'static,
    {
        ui.global::<Navigation>()
            .on_request_page_switch(move |tab_idx, page_idx| {
                handler(TabId(tab_idx as u32), PageId(page_idx as u32));
            });
    }

    fn on_side_bar_width_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(u64) + 'static,
    {
        ui.global::<Navigation>()
            .on_side_bar_width_changed(move |w| handler(w as u64));
    }

    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_handle = self.ui.clone();
        ui.global::<Navigation>().on_request_tab_switch(move |idx| {
            if let Some(ui) = ui_handle.upgrade() {
                let tabs = ui.global::<Navigation>().get_tabs();
                if let Some(tab) = tabs.row_data(idx as usize) {
                    handler(TabId(tab.id as u32));
                }
            }
        });
    }

    fn on_request_tab_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_handle = self.ui.clone();
        ui.global::<Navigation>().on_request_tab_close(move |idx| {
            if let Some(ui) = ui_handle.upgrade() {
                let tabs = ui.global::<Navigation>().get_tabs();
                if let Some(tab) = tabs.row_data(idx as usize) {
                    handler(TabId(tab.id as u32));
                }
            }
        });
    }

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
    for i in 0..model.row_count() {
        if let Some(mut item) = model.row_data(i) {
            if id_getter(&item) == id {
                updater(&mut item);
                model.set_row_data(i, item);
                return;
            }
        }
    }
}
