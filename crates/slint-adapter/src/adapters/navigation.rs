use crate::{AppWindow, Dashboard, Navigation, PageEntry};
use app_contracts::features::navigation::{NavigationUiBindings, NavigationUiPort, PageEntryDto};
use app_core::icons::Icons;
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Clone)]
pub struct NavigationUiAdapter {
    ui: slint::Weak<AppWindow>,
}

impl NavigationUiAdapter {
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

impl NavigationUiPort for NavigationUiAdapter {
    fn set_pages(&self, pages: Vec<PageEntryDto>) {
        self.with_ui(|ui| {
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
        });
    }

    fn get_active_tab_index(&self) -> i32 {
        if let Some(ui) = self.ui.upgrade() {
            return ui.global::<Navigation>().get_active_tab_index();
        }
        0
    }

    fn set_content_visible(&self, visible: bool) {
        self.with_ui(|ui| ui.global::<Navigation>().set_content_visible(visible));
    }

    fn set_active_tab_index(&self, index: i32) {
        self.with_ui(|ui| ui.global::<Navigation>().set_active_tab_index(index));
    }

    fn set_switch_transition(&self, from_index: i32, to_index: i32, progress: f32) {
        self.with_ui(|ui| {
            let nav = ui.global::<Navigation>();
            nav.set_switch_from_index(from_index);
            nav.set_switch_to_index(to_index);
            nav.set_switch_progress(progress);
        });
    }

    fn set_switch_progress(&self, progress: f32) {
        self.with_ui(|ui| ui.global::<Navigation>().set_switch_progress(progress));
    }
}

impl NavigationUiBindings for NavigationUiAdapter {
    fn on_request_tab_switch<F>(&self, handler: F)
    where
        F: Fn(i32) + 'static,
    {
        self.with_ui(move |ui| {
            ui.global::<Navigation>()
                .on_request_tab_switch(move |i| handler(i));
        });
    }
}
