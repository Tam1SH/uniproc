use app_core::actor::traits::Message;

pub fn tab_name_by_index(index: i32) -> &'static str {
    match index {
        0 => "Processes",
        1 => "Performance",
        2 => "Disk",
        3 => "Statistics",
        4 => "Startup apps",
        5 => "Users",
        6 => "Services",
        _ => "Unknown",
    }
}

#[derive(Clone, Debug)]
pub struct TabChanged {
    pub name: String,
}

impl Message for TabChanged {}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageEntryDto {
    pub id: i32,
    pub text: String,
    pub icon_key: String,
}

pub trait NavigationUiPort: 'static {
    fn set_pages(&self, pages: Vec<PageEntryDto>);
    fn get_active_tab_index(&self) -> i32;
    fn set_content_visible(&self, visible: bool);
    fn set_active_tab_index(&self, index: i32);
    fn set_switch_transition(&self, from_index: i32, to_index: i32, progress: f32);
    fn set_switch_progress(&self, progress: f32);
    fn set_side_bar_width(&self, width: u64);
}

pub trait NavigationUiBindings: 'static {
    fn on_request_tab_switch<F>(&self, handler: F)
    where
        F: Fn(i32) + 'static;
    fn on_side_bar_width_changed<F>(&self, handler: F)
    where
        F: Fn(u64) + 'static;
}
