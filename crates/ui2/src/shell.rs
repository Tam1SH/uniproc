use windows_reactor::{
    grid, Element, ElementExt, GridLength, Icon, NavViewItem, NavigationView,
    NavigationViewPaneDisplayMode, TitleBar,
};

pub fn shell_view(
    open: bool,
    selected_tag: &str,
    title_icon: Icon,
    items: Vec<(&str, &str, Icon)>,
    footer_items: Vec<(&str, &str, Icon)>,
    content: Element,
    on_toggle: impl Fn() + 'static,
    on_select: impl Fn(String) + 'static,
) -> Element {
    let title_bar = TitleBar::new("uniproc")
        .icon(title_icon)
        .pane_toggle_button_visible(true)
        .on_pane_toggle_requested(on_toggle);

    let to_nav_item = |(tag, label, icon): (&str, &str, Icon)| NavViewItem::new(label).tag(tag).icon(icon);
    let nav_items = items.into_iter().map(to_nav_item);
    let footer_nav_items = footer_items.into_iter().map(to_nav_item);

    let nav_view = NavigationView::new(nav_items, content)
        .footer_menu_items(footer_nav_items)
        .selected_tag(selected_tag)
        .on_selection_changed(on_select)
        .pane_open(open)
        .pane_display_mode(NavigationViewPaneDisplayMode::Left)
        .pane_toggle_button_visible(false)
        .back_button_visible(false)
        .settings_visible(false);

    grid((title_bar.grid_row(0), nav_view.grid_row(1)))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .into()
}
