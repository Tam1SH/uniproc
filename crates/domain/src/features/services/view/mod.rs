use crate::features::services::settings::ServiceSettings;
use app_contracts::features::services::{ServiceEntryDto, ServiceEntryVm};
use context::caches::strings::StringsProvider;
use context::settings::reactive::ReactiveSettingSubscription;
use dashmap::DashMap;
use slint::SharedString;
use std::collections::HashSet;
use widgets::table::flow::{SortState, TableDataBuilder, TableNode};
use widgets::table::layout::TableSettingsProvider;
use widgets::table::view::TableView;
use widgets::table::window::TableBatch;

struct ServiceTableSettingsAdapter(ServiceSettings);

impl TableSettingsProvider for ServiceTableSettingsAdapter {
    fn default_width(&self) -> anyhow::Result<u64> {
        Ok(self.0.columns().default_width_px().get())
    }
    fn initial_widths(&self) -> anyhow::Result<DashMap<String, u64>> {
        Ok(self.0.columns().widths_px().get())
    }
    fn min_widths(&self) -> anyhow::Result<DashMap<String, u64>> {
        Ok(self.0.columns().min_widths_px().get())
    }
    fn subscribe_widths<F>(&self, callback: F) -> ReactiveSettingSubscription
    where
        F: Fn(DashMap<String, u64>) + Send + Sync + 'static,
    {
        self.0.columns().widths_px().subscribe(callback)
    }
}

pub struct ServiceTable {
    pub view: TableView<
        ServiceEntryDto,
        ServiceEntryVm,
        SharedString,
        SharedString,
        SharedString,
        SharedString,
    >,
    settings: ServiceSettings,
    _sub: ReactiveSettingSubscription,
}

impl ServiceTable {
    pub fn new(settings: ServiceSettings) -> anyhow::Result<Self> {
        let mut view = TableView::new(
            SortState {
                field_id: Some("name".into()),
                descending: false,
            },
            50,
        );
        let sub = ServiceTableSettingsAdapter(settings.clone()).setup(&mut view.layout)?;
        Ok(Self {
            view,
            settings,
            _sub: sub,
        })
    }

    pub fn update_data(&mut self, items: Vec<ServiceEntryDto>) {
        self.view.flow.set_items(items);
        self.refresh();
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ServiceEntryDto> {
        self.view.flow.find(|dto| dto.name == name)
    }

    pub fn refresh(&mut self) {
        let mut builder = ServiceTableBuilder;
        let sort = self.view.flow.sort.clone();
        self.view.refresh_full(
            &mut builder,
            |nodes, _| {
                nodes.sort_by(|a, b| {
                    let res = match sort.field_id.as_ref().map(|s| s.as_str()) {
                        Some("pid") => a.vm.pid.cmp(&b.vm.pid),
                        Some("status") => a.vm.status.cmp(&b.vm.status),
                        _ => a.vm.name.cmp(&b.vm.name),
                    };
                    if sort.descending { res.reverse() } else { res }
                });
            },
            |vm| vm.name.clone(),
            |_| {},
            |_, _, _| {},
        );
    }

    pub fn resize_column(&mut self, id: String, new_width: u64) -> anyhow::Result<()> {
        let def_width = self.settings.columns().default_width_px().get();
        let min_w = self
            .settings
            .columns()
            .min_widths_px()
            .get()
            .get(&id)
            .map(|r| *r.value())
            .unwrap_or(def_width);

        self.settings.columns().patch_widths_px(|widths| {
            widths.insert(id, new_width.max(min_w));
        })?;

        Ok(())
    }

    pub fn column_widths(&self) -> Vec<(SharedString, u64)> {
        self.view
            .layout
            .widths
            .iter()
            .map(|(k, v)| (k.clone(), v.get()))
            .collect()
    }

    pub fn select(&mut self, name: SharedString, idx: usize) {
        self.view.flow.select(name, idx);
    }

    pub fn toggle_sort(&mut self, field_id: SharedString) {
        let current = &mut self.view.flow.sort;
        if current.field_id.as_ref() == Some(&field_id) {
            current.descending = !current.descending;
        } else {
            current.field_id = Some(field_id);
            current.descending = false;
        }
        self.refresh();
    }

    pub fn batch(&self) -> TableBatch<'_, ServiceEntryVm> {
        self.view.rows.batch()
    }
}

struct ServiceTableBuilder;
impl TableDataBuilder<ServiceEntryDto, ServiceEntryVm> for ServiceTableBuilder {
    fn build_tree(
        &mut self,
        items: &[ServiceEntryDto],
        _expanded: &HashSet<SharedString>,
        out: &mut Vec<TableNode<ServiceEntryVm>>,
    ) {
        let provider = StringsProvider::global();
        out.reserve(items.len());

        for item in items {
            out.push(TableNode {
                vm: ServiceEntryVm {
                    name: provider.get_stripped(&item.name),
                    display_name: provider.intern(&item.display_name),
                    pid: item.pid,
                    status: provider.intern(&item.status),
                    group: provider.intern(&item.group),
                    description: provider.intern(&item.description),
                },
                group_id: None,
                has_children: false,
                is_expanded: false,
                level: 0,
                children: vec![],
            });
        }
    }
}
