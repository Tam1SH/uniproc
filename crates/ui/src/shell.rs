use app_contracts::features::metrics::MetricsReducer;
use guicons::icon;
use guinea::router::LayoutCx;
use guinea::widgets::resize::{resize_handle, RESIZE_HANDLE_WIDTH};
use windows_reactor::{
    grid, hstack, text_block, vstack, Element, ElementExt, GridLength, HorizontalAlignment, Icon,
    NavViewItem, NavigationView, NavigationViewPaneDisplayMode, SetState, Thickness,
    TitleBar, VerticalAlignment,
};

use crate::format;
use crate::l10n::{use_tr, L10n};
use crate::theme::{size, space};
use crate::widgets::separator;
use crate::widgets::metric_chart::{
    metric_chart, metric_mini_bar, MetricChartKind, MetricChartStyle,
};

const MIN_SIDEBAR_WIDTH: f64 = 200.0;
const MAX_SIDEBAR_WIDTH: f64 = 500.0;
const SIDEBAR_SPARKLINE_HEIGHT: f64 = 28.0;

fn nav_items(l10n: &L10n) -> Vec<(&'static str, String, Icon)> {
    vec![
        (
            "processes",
            l10n.shell_nav_processes(),
            icon!(apps_list).size(size::NavIcon).build(),
        ),
        (
            "services",
            l10n.shell_nav_services(),
            icon!(puzzle).size(size::NavIcon).build(),
        ),
        (
            "wsl",
            l10n.shell_nav_wsl(),
            icon!(linux).size(size::NavIcon).build(),
        ),
    ]
}

fn footer_nav_items(l10n: &L10n) -> Vec<(&'static str, String, Icon)> {
    vec![(
        "settings",
        l10n.shell_nav_settings(),
        icon!(settings).size(size::NavIcon).build(),
    )]
}

fn metrics_pane_footer(cx: &mut LayoutCx, open: bool) -> Element {
    let scheme = cx.use_color_scheme();
    let (metrics_state, _) = cx.use_reducer::<MetricsReducer>();
    let machine = metrics_state.machine.ready();

    let cpu_detail = machine.map(|m| {
        format!(
            "{:.1} / {:.1} GHz",
            m.cpu_current_mhz as f64 / 1000.0,
            m.cpu_max_mhz as f64 / 1000.0
        )
    });
    let memory_detail = machine.map(|m| format::bytes(m.memory_total_bytes));

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
        let cpu = metric_mini_bar(scheme, MetricChartKind::Cpu, &metrics_state.cpu_history);
        let memory = metric_mini_bar(scheme, MetricChartKind::Memory, &metrics_state.memory_history);
        return vstack((separator(), cpu, memory, separator()))
            .spacing(space::Control)
            .padding(Thickness::xy(size::NavIcon, space::Control))
            .horizontal_alignment(HorizontalAlignment::Center)
            .into();
    }

    vstack((separator(), cpu_chart, memory_chart, separator()))
        .spacing(space::Control)
        .padding(Thickness::xy(space::Compact, space::Control))
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
    let l10n = use_tr(cx);
    let title_bar_icon = icon!(uniproc_logo).size(size::NavIcon).build_element();
    let title_bar = TitleBar::new(l10n.shell_window_title())
        .content(
            hstack((title_bar_icon, text_block(l10n.shell_window_title())))
                .spacing(space::Control)
                .vertical_alignment(VerticalAlignment::Center),
        )
        .pane_toggle_button_visible(false);

    let to_nav_item =
        |(tag, label, icon): (&str, String, Icon)| NavViewItem::new(label).tag(tag).icon(icon);
    let nav_items = nav_items(&l10n).into_iter().map(to_nav_item);
    let footer_nav_items = footer_nav_items(&l10n).into_iter().map(to_nav_item);

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
