use crate::core::actor::event_bus::EVENT_BUS;
use crate::core::actor::traits::Message;
use crate::core::reactor::Reactor;
use crate::features::Feature;
use crate::Navigation;
use crate::{messages, AppWindow};
use slint::{ComponentHandle, SharedString};
use std::time::Duration;

pub mod utils;

pub struct NavigationFeature;

impl Feature for NavigationFeature {
    fn install(self, _reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let ui_handle = ui.as_weak();

        ui.global::<Navigation>()
            .on_request_tab_switch(move |new_index| {
                let ui = match ui_handle.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };

                let tab_name = match new_index {
                    0 => "Processes",
                    1 => "Performance",
                    2 => "Network",
                    _ => "Unknown",
                };

                let event = TabChanged {
                    name: tab_name.to_string().into(),
                };
                EVENT_BUS.with(|bus| bus.publish(event));

                let nav = ui.global::<Navigation>();

                if nav.get_active_tab() == new_index {
                    return;
                }

                nav.set_content_visible(false);

                slint::Timer::single_shot(Duration::from_millis(60), move || {
                    let nav = ui.global::<Navigation>();

                    nav.set_active_tab(new_index);

                    slint::Timer::single_shot(Duration::from_millis(20), move || {
                        ui.global::<Navigation>().set_content_visible(true);
                    });
                });
            });

        Ok(())
    }
}

messages! {
    TabChanged {
        name: SharedString
    }
}
