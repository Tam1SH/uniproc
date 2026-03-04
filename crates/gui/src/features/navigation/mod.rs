use crate::core::actor::event_bus::EVENT_BUS;
use crate::core::actor::traits::Message;
use crate::core::reactor::Reactor;
use crate::features::navigation::utils::get_tab_name_by_index;
use crate::features::Feature;
use crate::shared::icons::Icons;
use crate::{messages, AppWindow, Dashboard};
use crate::{Navigation, PageEntry};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::Duration;

pub mod utils;

pub struct NavigationFeature;

impl Feature for NavigationFeature {
    fn install(self, _reactor: &mut Reactor, ui: &AppWindow) -> anyhow::Result<()> {
        let ui_handle = ui.as_weak();

        ui.global::<Dashboard>()
            .set_pages(ModelRc::new(VecModel::from(vec![
                PageEntry {
                    id: 0,
                    text: "Processes".into(),
                    icon: Icons::get("app"),
                },
                PageEntry {
                    id: 1,
                    text: "Performance".into(),
                    icon: Icons::get("data-area"),
                },
                PageEntry {
                    id: 2,
                    text: "Disk".into(),
                    icon: Icons::get("database"),
                },
                PageEntry {
                    id: 3,
                    text: "Statistics".into(),
                    icon: Icons::get("statistics"),
                },
                PageEntry {
                    id: 4,
                    text: "Startup apps".into(),
                    icon: Icons::get("dashed-settings"),
                },
                PageEntry {
                    id: 5,
                    text: "Users".into(),
                    icon: Icons::get("people"),
                },
                PageEntry {
                    id: 6,
                    text: "Services".into(),
                    icon: Icons::get("puzzle"),
                },
            ])));

        // pub fn get_tab_name_by_index(index: i32) -> &'static str {
        //     match index {
        //         0 => "Processes",
        //         1 => "Performance",
        //         2 => "Disk",
        //         3 => "Statistics",
        //         4 => "Startup apps",
        //         5 => "Users",
        //         6 => "Services",
        //         _ => "Unknown",
        //     }
        // }

        ui.global::<Navigation>()
            .on_request_tab_switch(move |new_index| {
                let ui = match ui_handle.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };

                let tab_name = get_tab_name_by_index(new_index);

                let event = TabChanged {
                    name: tab_name.to_string().into(),
                };
                EVENT_BUS.with(|bus| bus.publish(event));

                let nav = ui.global::<Navigation>();

                if nav.get_active_tab_index() == new_index {
                    return;
                }

                nav.set_content_visible(false);

                slint::Timer::single_shot(Duration::from_millis(60), move || {
                    let nav = ui.global::<Navigation>();

                    nav.set_active_tab_index(new_index);

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
