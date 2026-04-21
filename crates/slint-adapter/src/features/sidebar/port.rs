use crate::features::sidebar::UiSidebarAdapter;
use app_contracts::features::sidebar::UiSidebarPort;
use macros::slint_port_adapter;
use slint::ComponentHandle;
use slint::private_unstable_api::re_exports::Coord;

#[slint_port_adapter(window = AppWindow)]
impl UiSidebarPort for UiSidebarAdapter {
    fn set_switch_transition(&self, ui: &AppWindow, from_index: i32, to_index: i32, progress: f32) {
        let sidebar = ui.global::<crate::Sidebar>();
        sidebar.set_switch_from_index(from_index);
        sidebar.set_switch_to_index(to_index);
        sidebar.set_switch_progress(progress);
    }

    fn set_side_bar_width(&self, ui: &AppWindow, width: u64) {
        ui.global::<crate::Sidebar>()
            .set_side_bar_width(width as Coord)
    }
}
