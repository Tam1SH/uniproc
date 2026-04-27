use crate::features::tabs::UiTabsAdapter;
use app_contracts::features::tabs::{AvailableContextDescriptor, TabContextKey, TabDescriptor, UiTabsPort};
use context::page_status::PageStatus;
use context::icons::Icons;
use macros::slint_port_adapter;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

#[slint_port_adapter(window = AppWindow)]
impl UiTabsPort for UiTabsAdapter {
    fn set_tabs(&self, ui: &AppWindow, tabs: Vec<TabDescriptor>) {
        let slint_tabs: Vec<_> = tabs
            .into_iter()
            .map(|tab| {
                let pages: Vec<_> = tab
                    .pages
                    .into_iter()
                    .map(|p| crate::PageData {
                        path: p.path.into(),
                        route_segment: p.route_segment.into(),
                        text: p.text.into(),
                        icon: Icons::get(&p.icon_key),
                        status: p.status.into(),
                        error_msg: p.error_msg.into(),
                    })
                    .collect();

                crate::TabData {
                    id: 0,
                    context_key: tab.context_key.0.to_string().into(),
                    title: tab.title.into(),
                    icon: Icons::get(&tab.icon_key),
                    pages: ModelRc::new(VecModel::from(pages)),
                    status: tab.status.into(),
                    error_msg: tab.error_msg.into(),
                    is_closable: tab.is_closable,
                }
            })
            .collect();
        let slint_tabs: Vec<_> = slint_tabs
            .into_iter()
            .enumerate()
            .map(|(i, mut tab)| {
                tab.id = i as i32;
                tab
            })
            .collect();

        ui.global::<crate::Tabs>()
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

        ui.global::<crate::Tabs>()
            .set_available_contexts(ModelRc::new(VecModel::from(slint_contexts)));
    }

    fn set_active_context(&self, ui: &AppWindow, context_key: TabContextKey) {
        let tabs = ui.global::<crate::Tabs>();
        if let Some(idx) = tabs
            .get_tabs()
            .iter()
            .position(|t| t.context_key.as_str() == context_key.0.as_ref())
        {
            tabs.set_active_tab_index(idx as i32);
        }
    }

    fn set_active_page(&self, ui: &AppWindow, context_key: TabContextKey, route_segment: String) {
        let tabs = ui.global::<crate::Tabs>();
        if let Some(tab) = tabs
            .get_tabs()
            .iter()
            .find(|t| t.context_key.as_str() == context_key.0.as_ref())
        {
            if let Some(idx) = tab
                .pages
                .iter()
                .position(|p| p.route_segment.as_str() == route_segment.as_str())
            {
                tabs.set_active_page_index(idx as i32);
            }
        }
    }

    fn set_route_status(
        &self,
        ui: &AppWindow,
        context_key: TabContextKey,
        route_segment: String,
        status: PageStatus,
    ) {
        if let Some(tab) = ui
            .global::<crate::Tabs>()
            .get_tabs()
            .iter()
            .find(|t| t.context_key.as_str() == context_key.0.as_ref())
        {
            if let Some(idx) = tab
                .pages
                .iter()
                .position(|p| p.route_segment.as_str() == route_segment.as_str())
            {
                if let Some(mut row) = tab.pages.row_data(idx) {
                    row.status = status.into();
                    tab.pages.set_row_data(idx, row);
                }
            }
        }
    }

    fn set_route_error(
        &self,
        ui: &AppWindow,
        context_key: TabContextKey,
        route_segment: String,
        msg: String,
    ) {
        if let Some(tab) = ui
            .global::<crate::Tabs>()
            .get_tabs()
            .iter()
            .find(|t| t.context_key.as_str() == context_key.0.as_ref())
        {
            if let Some(idx) = tab
                .pages
                .iter()
                .position(|p| p.route_segment.as_str() == route_segment.as_str())
            {
                if let Some(mut row) = tab.pages.row_data(idx) {
                    row.error_msg = msg.into();
                    tab.pages.set_row_data(idx, row);
                }
            }
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
