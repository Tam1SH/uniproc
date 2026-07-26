use app_contracts2::icons;
use windows_reactor::{
    border, grid, Color, Element, ElementExt, GridLength, HorizontalAlignment, Icon, NavViewItem,
    NavigationView, NavigationViewPaneDisplayMode, PointerEventInfo, Thickness, TitleBar,
};

const NAV_ICON_SIZE: f64 = 20.0;
const MIN_SIDEBAR_WIDTH: f64 = 200.0;
const MAX_SIDEBAR_WIDTH: f64 = 500.0;
const RESIZE_HANDLE_WIDTH: f64 = 6.0;

fn icon_for(key: icons::IconKey) -> Icon {
    let path = icons::path_for(key).expect("icon key must resolve to a path");
    guicons::windows_reactor::icon_from_path(path, NAV_ICON_SIZE, NAV_ICON_SIZE)
}

fn nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![
        ("processes", "Processes", icon_for(icons::keys::APPS_LIST)),
        ("services", "Services", icon_for(icons::keys::PUZZLE)),
    ]
}

fn footer_nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![("settings", "Settings", icon_for(icons::keys::SETTINGS))]
}

pub fn shell_view(
    open: bool,
    selected_tag: &str,
    width: f64,
    content: Element,
    on_toggle: impl Fn() + 'static,
    on_select: impl Fn(String) + 'static,
    on_resize: impl Fn(f64) + 'static,
) -> Element {
    let title_bar = TitleBar::new("uniproc")
        .icon(icon_for(icons::keys::UNIPROC_LOGO))
        .pane_toggle_button_visible(true)
        .on_pane_toggle_requested(on_toggle);

    let to_nav_item = |(tag, label, icon): (&str, &str, Icon)| NavViewItem::new(label).tag(tag).icon(icon);
    let nav_items = nav_items().into_iter().map(to_nav_item);
    let footer_nav_items = footer_nav_items().into_iter().map(to_nav_item);

    let nav_view = NavigationView::new(nav_items, content)
        .footer_menu_items(footer_nav_items)
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
        .background(Color { a: 40, r: 128, g: 128, b: 128 })
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
