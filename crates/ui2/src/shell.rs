use windows_reactor::{
    border, grid, Element, ElementExt, GridLength, HorizontalAlignment, Icon, NavViewItem,
    NavigationView, NavigationViewPaneDisplayMode, PointerEventInfo, Thickness, TitleBar,
};

const MIN_SIDEBAR_WIDTH: f64 = 200.0;
const MAX_SIDEBAR_WIDTH: f64 = 500.0;
const RESIZE_HANDLE_WIDTH: f64 = 6.0;

pub fn shell_view(
    open: bool,
    selected_tag: &str,
    width: f64,
    title_icon: Icon,
    items: Vec<(&str, &str, Icon)>,
    content: Element,
    on_toggle: impl Fn() + 'static,
    on_select: impl Fn(String) + 'static,
    on_resize: impl Fn(f64) + 'static,
) -> Element {
    let title_bar = TitleBar::new("uniproc")
        .icon(title_icon)
        .pane_toggle_button_visible(true)
        .on_pane_toggle_requested(on_toggle);

    let nav_items = items
        .into_iter()
        .map(|(tag, label, icon)| NavViewItem::new(label).tag(tag).icon(icon));

    let nav_view = NavigationView::new(nav_items, content)
        .selected_tag(selected_tag)
        .on_selection_changed(on_select)
        .pane_open(open)
        .pane_display_mode(NavigationViewPaneDisplayMode::Left)
        .pane_toggle_button_visible(false)
        .back_button_visible(false)
        .settings_visible(false)
        .open_pane_length(width);

    // Overlaid on top of the NavigationView (same grid cell) rather than a
    // sibling column: NavigationView owns its pane/content split internally,
    // there's nowhere else to attach a splitter (WinUI has none built in -
    // microsoft-ui-xaml#190). Tracks the pane edge via a left margin equal to
    // the current width; `on_pointer_moved`'s `x` is relative to the handle's
    // own (unmoved-this-frame) bounds, so it doubles as the drag delta.
    let resize_handle = border(Element::Empty)
        .width(RESIZE_HANDLE_WIDTH)
        .horizontal_alignment(HorizontalAlignment::Left)
        .margin(Thickness {
            left: width - RESIZE_HANDLE_WIDTH / 2.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        })
        .on_pointer_pressed(|_: PointerEventInfo| {})
        .on_pointer_moved(move |info: PointerEventInfo| {
            if info.is_left_button_pressed {
                on_resize((width + info.x).clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH));
            }
        })
        .grid_row(1);

    grid((title_bar.grid_row(0), nav_view.grid_row(1), resize_handle))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}
