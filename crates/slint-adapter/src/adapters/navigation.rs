use crate::{AppWindow, Dashboard, Navigation, PageEntry};
use app_contracts::features::navigation::{NavigationUiBindings, NavigationUiPort, PageEntryDto};
use app_core::icons::Icons;
use macros::ui_adapter;
use slint::private_unstable_api::re_exports::Coord;
use slint::{ComponentHandle, ModelRc, VecModel};

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
    fn set_pages(&self, ui: &AppWindow, pages: Vec<PageEntryDto>) {
        let pages = pages
            .into_iter()
            .map(|p| PageEntry {
                id: p.id,
                text: p.text.into(),
                icon: Icons::get(&p.icon_key),
            })
            .collect::<Vec<_>>();

        ui.global::<Dashboard>()
            .set_pages(ModelRc::new(VecModel::from(pages)));
    }

    #[default(0)]
    fn get_active_tab_index(&self, ui: &AppWindow) -> i32 {
        ui.global::<Navigation>().get_active_tab_index()
    }

    fn set_content_visible(&self, ui: &AppWindow, visible: bool) {
        ui.global::<Navigation>().set_content_visible(visible)
    }

    fn set_active_tab_index(&self, ui: &AppWindow, index: i32) {
        ui.global::<Navigation>().set_active_tab_index(index)
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

    fn set_side_bar_width(&self, ui: &AppWindow, width: u64) {
        ui.global::<Navigation>().set_side_bar_width(width as Coord)
    }
}

#[ui_adapter]
impl NavigationUiBindings for NavigationUiAdapter {
    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(i32) + 'static,
    {
        ui.global::<Navigation>().on_request_tab_switch(handler);
    }

    fn on_side_bar_width_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(u64) + 'static,
    {
        ui.global::<Navigation>()
            .on_side_bar_width_changed(move |w| handler(w as u64));
    }
}
