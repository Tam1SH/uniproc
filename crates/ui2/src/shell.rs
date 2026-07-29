use guinea::widgets::resize::{resize_handle, RESIZE_HANDLE_WIDTH};
use guicons::icon;
use windows_reactor::{
    grid, Element, ElementExt, GridLength, Icon, NavViewItem,
    NavigationView, NavigationViewPaneDisplayMode, RenderCx, SetState, Thickness, TitleBar,
};

const NAV_ICON_SIZE: f64 = 20.0;
const MIN_SIDEBAR_WIDTH: f64 = 200.0;
const MAX_SIDEBAR_WIDTH: f64 = 500.0;

fn nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![
        ("processes", "Processes", icon!(apps_list).size(NAV_ICON_SIZE).build()),
        ("services", "Services", icon!(puzzle).size(NAV_ICON_SIZE).build()),
    ]
}

fn footer_nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![("settings", "Settings", icon!(settings).size(NAV_ICON_SIZE).build())]
}

pub fn shell_view(
    cx: &mut RenderCx,
    open: bool,
    selected_tag: &str,
    width: f64,
    content: Element,
    on_select: impl Fn(String) + 'static,
    set_width: SetState<f64>,
) -> Element {
    let title_bar = TitleBar::new("uniproc")
        .icon(icon!(uniproc_logo).size(NAV_ICON_SIZE).build())
        .pane_toggle_button_visible(false);

    let to_nav_item =
        |(tag, label, icon): (&str, &str, Icon)| NavViewItem::new(label).tag(tag).icon(icon);
    let nav_items = nav_items().into_iter().map(to_nav_item);
    let footer_nav_items = footer_nav_items().into_iter().map(to_nav_item);

    let nav_view = NavigationView::new(nav_items, content)
        .footer_menu_items(footer_nav_items)
        .selected_tag(selected_tag)
        .on_selection_changed(on_select)
        .pane_open(open)
        .pane_display_mode(NavigationViewPaneDisplayMode::Left)
        .pane_toggle_button_visible(true)
        .back_button_visible(false)
        .settings_visible(false)
        .open_pane_length(width);

    // The hook registers unconditionally (stable hook ordering); the element
    // itself only exists while the pane is open - a collapsed pane has a fixed
    // compact width, so there is nothing to resize.
    let sidebar_resize_handle = resize_handle(cx, width, set_width)
        .min(MIN_SIDEBAR_WIDTH)
        .max(MAX_SIDEBAR_WIDTH);
    let sidebar_resize_handle = if open {
        sidebar_resize_handle
            .build()
            .margin(Thickness {
                left: width - RESIZE_HANDLE_WIDTH / 2.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            })
            .grid_row(1)
    } else {
        Element::Empty
    };

    grid((
        title_bar.grid_row(0),
        nav_view.grid_row(1),
        sidebar_resize_handle,
    ))
    .rows([GridLength::Auto, GridLength::Star(1.0)])
    .columns([GridLength::Star(1.0)])
    .into()
}
