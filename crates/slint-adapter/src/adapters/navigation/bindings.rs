use crate::adapters::navigation::NavigationUiAdapter;
use app_contracts::features::navigation::NavigationUiBindings;
use context::page_status::{PageId, TabId};
use macros::slint_bindings_adapter;
use slint::{ComponentHandle, Model};

#[slint_bindings_adapter(window = AppWindow)]
impl NavigationUiBindings for NavigationUiAdapter {
    fn on_request_page_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId, PageId) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_request_page_switch(move |t, p| handler(TabId(t as u32), PageId(p as u32)));
    }

    fn on_side_bar_width_changed<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(u64) + 'static,
    {
        ui.global::<crate::Navigation>()
            .on_side_bar_width_changed(move |w| handler(w as u64));
    }

    fn on_request_tab_switch<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_h = self.ui.clone();
        ui.global::<crate::Navigation>()
            .on_request_tab_switch(move |idx| {
                if let Some(tab) = ui_h.upgrade().and_then(|ui| {
                    ui.global::<crate::Navigation>()
                        .get_tabs()
                        .row_data(idx as usize)
                }) {
                    handler(TabId(tab.id as u32));
                }
            });
    }

    fn on_request_tab_close<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn(TabId) + 'static,
    {
        let ui_h = self.ui.clone();
        ui.global::<crate::Navigation>()
            .on_request_tab_close(move |idx| {
                if let Some(tab) = ui_h.upgrade().and_then(|ui| {
                    ui.global::<crate::Navigation>()
                        .get_tabs()
                        .row_data(idx as usize)
                }) {
                    handler(TabId(tab.id as u32));
                }
            });
    }

    fn on_request_tab_add<F>(&self, ui: &AppWindow, handler: F)
    where
        F: Fn() + 'static,
    {
        ui.global::<crate::Navigation>().on_request_tab_add(handler);
    }
}
