use macros::slint_port;

#[slint_port(global = "Sidebar")]
pub trait UiSidebarPort: 'static {
    #[manual]
    fn set_switch_transition(&self, from_index: i32, to_index: i32, progress: f32);
    #[manual]
    fn set_side_bar_width(&self, width: u64);
    fn set_switch_progress(&self, progress: f32);
    fn set_content_visible(&self, visible: bool);
}
