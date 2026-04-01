use crate::features::navigation::settings::NavigationSettings;
use app_contracts::features::navigation::{
    NavigationUiPort, PageActivated, TabActivated, TabDescriptor,
};
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::Message;
use app_core::actor::traits::{Context, Handler};
use app_core::app::Window;
use app_core::messages;
use context::page_status::{
    PageId, PageStatusChanged, PageStatusRegistry, TabId, TabStatusChanged,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::instrument;
use tracing::{debug, info, warn};

messages! {
    RequestPageSwitch(TabId, PageId),
    RequestTabSwitch(TabId),
    RequestTabClose(TabId),
    RequestTabAdd,
    SideBarWidthChanged(u64),
}

pub struct NavigationActor<P: NavigationUiPort + Clone> {
    ui_port: P,
    registry: Arc<PageStatusRegistry>,

    tabs: Vec<TabDescriptor>,
    active_tab_id: TabId,

    tab_active_pages: HashMap<TabId, PageId>,

    anim_token: Arc<AtomicU64>,
    switch_duration: Duration,
    hide_delay: Duration,
    show_delay: Duration,
}

impl<P: NavigationUiPort + Clone> NavigationActor<P> {
    pub fn new(
        ui_port: P,
        registry: Arc<PageStatusRegistry>,
        tabs: Vec<TabDescriptor>,
        settings: &NavigationSettings,
    ) -> Self {
        let active_tab_id = tabs.first().map(|t| t.id).unwrap_or_default();
        let mut tab_active_pages = HashMap::new();

        for tab in &tabs {
            if let Some(page) = tab.pages.first() {
                tab_active_pages.insert(tab.id, page.id);
            }
        }

        Self {
            ui_port,
            registry,
            tabs,
            active_tab_id,
            tab_active_pages,
            anim_token: Arc::new(AtomicU64::new(0)),
            switch_duration: Duration::from_millis(600),
            hide_delay: Duration::from_millis(settings.switch_hide_delay_ms().get()),
            show_delay: Duration::from_millis(settings.switch_show_delay_ms().get()),
        }
    }

    fn sync_all_statuses(&self) {
        for tab in &self.tabs {
            let tab_state = self.registry.get_tab_state(tab.id);
            self.ui_port.set_tab_status(tab.id, tab_state.status);
            self.ui_port.set_tab_error(tab.id, tab_state.error_msg);

            for page in &tab.pages {
                let page_state = self.registry.get_page_state(tab.id, page.id);
                self.ui_port
                    .set_page_status(tab.id, page.id, page_state.status);
                self.ui_port
                    .set_page_error(tab.id, page.id, page_state.error_msg);
            }
        }
    }

    fn run_animation_step(
        ui: P,
        token_ref: Arc<AtomicU64>,
        target_token: u64,
        start: Instant,
        duration: Duration,
    ) {
        slint::Timer::single_shot(Duration::from_millis(16), move || {
            if token_ref.load(Ordering::SeqCst) != target_token {
                return;
            }

            let elapsed = start.elapsed().as_secs_f32();
            let total = duration.as_secs_f32().max(0.001);
            let t = (elapsed / total).clamp(0.0, 1.0);

            let eased = if t < 0.5 {
                8.0 * t * t * t * t
            } else {
                1.0 - f32::powi(-2.0 * t + 2.0, 4) / 2.0
            };

            ui.set_switch_progress(eased);

            if t < 1.0 {
                Self::run_animation_step(ui, token_ref, target_token, start, duration);
            } else {
                ui.set_switch_progress(1.0);
            }
        });
    }

    #[instrument(skip(self), fields(tab_id = ?tab_id, page_id = ?page_id))]
    fn perform_page_switch(&mut self, tab_id: TabId, page_id: PageId) {
        let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) else {
            warn!("Switch failed: Tab not found");
            return;
        };
        let Some((new_index, _)) = tab.pages.iter().enumerate().find(|(_, p)| p.id == page_id)
        else {
            warn!("Switch failed: Page not found in tab");
            return;
        };

        let old_page_id = self.tab_active_pages.get(&tab_id).cloned();
        if Some(page_id) == old_page_id && tab_id == self.active_tab_id {
            debug!("Switch skipped: already on this page");
            return;
        }

        let from_index = if tab_id == self.active_tab_id {
            tab.pages
                .iter()
                .position(|p| Some(p.id) == old_page_id)
                .unwrap_or(0) as i32
        } else {
            -1
        };

        info!(
            from_tab = ?self.active_tab_id,
            to_tab = ?tab_id,
            from_page = ?old_page_id,
            to_page = ?page_id,
            "Switching page"
        );

        self.tab_active_pages.insert(tab_id, page_id);
        self.active_tab_id = tab_id;

        EventBus::publish(TabActivated { tab_id });
        EventBus::publish(PageActivated { tab_id, page_id });

        self.ui_port.set_active_tab(tab_id);
        self.ui_port.set_active_page(tab_id, page_id);

        let page_state = self.registry.get_page_state(tab_id, page_id);
        self.ui_port
            .set_page_status(tab_id, page_id, page_state.status);

        if from_index != -1 {
            debug!(from_index, new_index, "Starting transition animation");
            self.ui_port
                .set_switch_transition(from_index, new_index as i32, 0.0);
            let next_token = self.anim_token.fetch_add(1, Ordering::SeqCst) + 1;

            Self::run_animation_step(
                self.ui_port.clone(),
                self.anim_token.clone(),
                next_token,
                Instant::now(),
                self.switch_duration,
            );
        }

        self.ui_port.set_content_visible(false);

        let h_delay = self.hide_delay;
        let s_delay = self.show_delay;
        let ui = self.ui_port.clone();

        slint::Timer::single_shot(h_delay, move || {
            let ui2 = ui.clone();
            slint::Timer::single_shot(s_delay, move || {
                debug!("Setting content visible after delay");
                ui2.set_content_visible(true);
            });
        });
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<RequestPageSwitch, TWindow>
    for NavigationActor<P>
{
    fn handle(&mut self, msg: RequestPageSwitch, _ctx: &Context<Self, TWindow>) {
        info!("aya");
        self.perform_page_switch(msg.0, msg.1);
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<RequestTabSwitch, TWindow>
    for NavigationActor<P>
{
    #[instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: RequestTabSwitch, _ctx: &Context<Self, TWindow>) {
        let tab_id = msg.0;
        let page_id = self
            .tab_active_pages
            .get(&tab_id)
            .cloned()
            .or_else(|| {
                self.tabs
                    .iter()
                    .find(|t| t.id == tab_id)
                    .and_then(|t| t.pages.first())
                    .map(|p| p.id)
            })
            .unwrap_or_default();

        self.perform_page_switch(tab_id, page_id);
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<PageStatusChanged, TWindow>
    for NavigationActor<P>
{
    #[instrument(skip(self, _ctx), fields(tab_id = ?msg.tab_id, page_id = ?msg.page_id, status = ?msg.status))]
    fn handle(&mut self, msg: PageStatusChanged, _ctx: &Context<Self, TWindow>) {
        self.ui_port
            .set_page_status(msg.tab_id, msg.page_id, msg.status);
        if let Some(err) = msg.error {
            warn!(error = %err, "Page error reported");
            self.ui_port.set_page_error(msg.tab_id, msg.page_id, err);
        }
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<TabStatusChanged, TWindow>
    for NavigationActor<P>
{
    #[instrument(skip(self, _ctx), fields(tab_id = ?msg.tab_id, status = ?msg.status))]
    fn handle(&mut self, msg: TabStatusChanged, _ctx: &Context<Self, TWindow>) {
        self.ui_port.set_tab_status(msg.tab_id, msg.status);
        if let Some(err) = msg.error {
            warn!(error = %err, "Tab error reported");
            self.ui_port.set_tab_error(msg.tab_id, err);
        }
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<SideBarWidthChanged, TWindow>
    for NavigationActor<P>
{
    fn handle(&mut self, msg: SideBarWidthChanged, _ctx: &Context<Self, TWindow>) {
        self.ui_port.set_side_bar_width(msg.0);
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<RequestTabClose, TWindow>
    for NavigationActor<P>
{
    fn handle(&mut self, _msg: RequestTabClose, _ctx: &Context<Self, TWindow>) {
        // TODO: Implement tab closing logic
    }
}

impl<P: NavigationUiPort + Clone, TWindow: Window> Handler<RequestTabAdd, TWindow>
    for NavigationActor<P>
{
    fn handle(&mut self, _msg: RequestTabAdd, _ctx: &Context<Self, TWindow>) {
        // TODO: Implement tab adding logic
    }
}
