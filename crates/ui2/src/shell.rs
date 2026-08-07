use app_contracts2::features::metrics::MetricsReducer;
use guicons::icon;
use guinea::router::LayoutCx;
use guinea::widgets::resize::{resize_handle, RESIZE_HANDLE_WIDTH};
use windows_reactor::{
    grid, hstack, text_block, vstack, Element, ElementExt, GridLength, HorizontalAlignment, Icon,
    NavViewItem, NavigationView, NavigationViewPaneDisplayMode, SetState, Shape, Thickness,
    TitleBar, VerticalAlignment,
};

use crate::table_styles;
use crate::widgets::metric_chart::{
    metric_chart, metric_mini_bar, MetricChartKind, MetricChartStyle,
};

const NAV_ICON_SIZE: f64 = 20.0;
const MIN_SIDEBAR_WIDTH: f64 = 200.0;
const MAX_SIDEBAR_WIDTH: f64 = 500.0;
/// Height of each sidebar metric tile's sparkline - tall enough to read a
/// trend at a glance, short enough that two tiles plus their labels still
/// fit comfortably in the pane footer without crowding Settings below them.
const SIDEBAR_SPARKLINE_HEIGHT: f64 = 28.0;

fn nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![
        (
            "processes",
            "Processes",
            icon!(apps_list).size(NAV_ICON_SIZE).build(),
        ),
        (
            "services",
            "Services",
            icon!(puzzle).size(NAV_ICON_SIZE).build(),
        ),
    ]
}

fn footer_nav_items() -> Vec<(&'static str, &'static str, Icon)> {
    vec![(
        "settings",
        "Settings",
        icon!(settings).size(NAV_ICON_SIZE).build(),
    )]
}

fn separator() -> Element {
    Shape::rectangle()
        .fill(table_styles::TABLE_STYLES.separator_color)
        .height(1.0)
        .into()
}

/// CPU/Memory indicators shown in the nav pane's footer, above Settings -
/// visible from every page, not just Processes, since machine load isn't a
/// fact about the Processes page specifically.
///
/// The open pane shows full labeled sparkline tiles; the collapsed
/// (icon-only, `LeftCompact`) pane has no width for a label or a trend
/// line, so it falls back to a vertical fill bar per metric.
fn metrics_pane_footer(cx: &mut LayoutCx, open: bool) -> Element {
    let (metrics_state, _) = cx.use_reducer::<MetricsReducer>();
    let machine = metrics_state.machine.ready();

    // The sidebar has real room next to the label, unlike the table's
    // column headers - the physical spec (clock speed, total memory) is
    // static context that's cheap to show alongside the live percentage.
    let cpu_detail = machine.map(|m| {
        format!(
            "{:.1} / {:.1} GHz",
            m.cpu_current_mhz as f64 / 1000.0,
            m.cpu_max_mhz as f64 / 1000.0
        )
    });
    let memory_detail = machine.map(|m| table_styles::format_bytes(m.memory_total_bytes));

    // `metric_chart` mounts a swap-chain-backed chart and registers its own
    // hooks (`use_state`/`use_ref`) via `cx` internally. Hooks must be
    // called in the same order on every render regardless of branching, so
    // this is called unconditionally even in the collapsed case where its
    // result is discarded in favor of `metric_mini_bar` - skipping the call
    // there would shift every hook slot registered after it and corrupt
    // state (or panic) the moment the pane toggles.
    let cpu_chart = metric_chart(
        cx,
        MetricChartKind::Cpu,
        &metrics_state.cpu_history,
        SIDEBAR_SPARKLINE_HEIGHT,
        MetricChartStyle::SPARKLINE,
        cpu_detail,
    );
    let memory_chart = metric_chart(
        cx,
        MetricChartKind::Memory,
        &metrics_state.memory_history,
        SIDEBAR_SPARKLINE_HEIGHT,
        MetricChartStyle::SPARKLINE,
        memory_detail,
    );

    if !open {
        let cpu = metric_mini_bar(MetricChartKind::Cpu, &metrics_state.cpu_history);
        let memory = metric_mini_bar(MetricChartKind::Memory, &metrics_state.memory_history);
        // `Stretch` here, not `Center` - a `Center`-aligned StackPanel
        // auto-sizes to its widest child (the 6px bars) and centers *that*,
        // leaving nothing to center within. Stretching the panel to the
        // full compact-pane width is what gives the per-bar `Center`
        // alignment (set in `metric_mini_bar`) any room to actually apply.
        return vstack((separator(), cpu, memory, separator()))
            .spacing(8.0)
            .padding(Thickness::xy(20.0, 8.0))
            .horizontal_alignment(HorizontalAlignment::Center)
            .into();
    }

    // `PaneFooter` doesn't stretch its content the way a Grid/StackPanel
    // child would - unlike the Processes page's chart card (which inherits
    // a real width from its Star-column grid cell), nothing here forces
    // this subtree wide, so the swap-chain-backed sparkline can end up
    // measured at zero width without an explicit stretch.

    vstack((separator(), cpu_chart, memory_chart, separator()))
        .spacing(8.0)
        .padding(Thickness::xy(4.0, 8.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .into()
}

pub fn shell_view(
    cx: &mut LayoutCx,
    open: bool,
    selected_tag: &str,
    width: f64,
    content: Element,
    on_select: impl Fn(String) + 'static,
    set_width: SetState<f64>,
    set_open: SetState<bool>,
) -> Element {
    let title_bar_icon = icon!(uniproc_logo).size(NAV_ICON_SIZE).build_element();
    let title_bar = TitleBar::new("uniproc")
        .content(
            hstack((title_bar_icon, text_block("uniproc")))
                .spacing(8.0)
                .vertical_alignment(VerticalAlignment::Center),
        )
        .pane_toggle_button_visible(false);

    let to_nav_item =
        |(tag, label, icon): (&str, &str, Icon)| NavViewItem::new(label).tag(tag).icon(icon);
    let nav_items = nav_items().into_iter().map(to_nav_item);
    let footer_nav_items = footer_nav_items().into_iter().map(to_nav_item);

    // `IsPaneOpen` is a one-way (app -> native) prop, so the native
    // pane-toggle button flipping it never round-trips back into app state
    // on its own. `NavigationPaneOpenChanged` (WinUI's own
    // `PaneOpenChanged` wired through upstream's explicit-capture rework)
    // reports the pane's actual open state whenever it changes, including
    // our own `.pane_open(open)` call above echoing back synchronously
    // within the very same render pass that set it. Forwarding that echo
    // into `set_open` re-enters the reconciler mid-pass and trips its
    // state-dirty invariant (`a state-dirty component was not re-rendered
    // by the pass`). `last_known_open` is resynced to the authoritative
    // `open` prop every render, so only a transition genuinely different
    // from what we just rendered - i.e. a real native/user-driven change -
    // gets forwarded.
    let last_known_open = cx.use_ref(open);
    *last_known_open.borrow_mut() = open;
    let last_known_open_on_change = last_known_open.clone();
    let set_open_on_change = set_open.clone();

    let nav_view = NavigationView::new(nav_items, content)
        .footer_menu_items(footer_nav_items)
        .pane_footer(metrics_pane_footer(cx, open))
        .selected_tag(selected_tag)
        .on_selection_changed(on_select)
        .pane_open(open)
        .pane_display_mode(NavigationViewPaneDisplayMode::Left)
        .pane_toggle_button_visible(true)
        .on_pane_open_changed(move |is_open: bool| {
            if *last_known_open_on_change.borrow() != is_open {
                *last_known_open_on_change.borrow_mut() = is_open;
                set_open_on_change.call(is_open);
            }
        })
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
