use crate::processes_impl::domain::snapshot::BridgeSnapshot;
use crate::processes_impl::domain::table_builder::ProcessTreeBuilder;
use crate::processes_impl::services::metadata::ProcessMetadataService;
use crate::processes_impl::settings::ProcessSettings;
use app_contracts::features::processes::{
    FieldDefDto, FieldMetadata, ProcessEntryVm, ProcessNodeDto,
};
use app_core::settings::reactive::ReactiveSettingSubscription;
use app_table::flow::{SortState, TableNode};
use app_table::layout::TableSettingsProvider;
use app_table::view::TableView;
use app_table::window::TableBatch;
use dashmap::DashMap;
use slint::SharedString;

struct ProcessTableSettingsAdapter(ProcessSettings);

impl TableSettingsProvider for ProcessTableSettingsAdapter {
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

pub struct ProcessTable {
    view: TableView<ProcessNodeDto, ProcessEntryVm, u32, SharedString, SharedString, SharedString>,
    settings: ProcessSettings,
    grouping_scratchpad: Vec<(SharedString, usize)>,
    _sub: ReactiveSettingSubscription,
    header_columns: Vec<FieldDefDto>,
}

impl ProcessTable {
    pub fn new(settings: ProcessSettings) -> anyhow::Result<Self> {
        let mut view = TableView::new(
            SortState {
                field_id: Some("cpu".to_string().into()),
                descending: true,
            },
            50,
        );

        let sub = ProcessTableSettingsAdapter(settings.clone()).setup(&mut view.layout)?;

        Ok(Self {
            view,
            settings,
            grouping_scratchpad: Vec::with_capacity(1024),
            _sub: sub,
            header_columns: Vec::new(),
        })
    }

    pub fn handle_snapshot(
        &mut self,
        snapshot: BridgeSnapshot,
        metadata: &mut ProcessMetadataService,
    ) -> anyhow::Result<()> {
        self.view.flow.set_items(snapshot.processes);
        self.build_header(snapshot.column_defs);
        self.refresh(metadata)
    }

    fn build_header(&mut self, snapshot_metrics: Vec<FieldDefDto>) {
        let mut columns = Vec::new();

        let mut name_def = FieldDefDto::default();
        name_def.id = "name".into();
        name_def.label = "Name".into();
        columns.push(name_def);

        for metric in snapshot_metrics {
            if !columns.iter().any(|c| c.id == metric.id) {
                columns.push(metric);
            }
        }

        self.header_columns = columns;
    }

    pub fn refresh(&mut self, metadata: &mut ProcessMetadataService) -> anyhow::Result<()> {
        let mut builder = ProcessTreeBuilder {
            metadata,
            show_icons: self.settings.show_icons().get(),
            grouping_scratchpad: &mut self.grouping_scratchpad,
        };
        let sort_state = self.view.flow.sort.clone();
        self.view.refresh_full(
            &mut builder,
            |nodes, _| sort_nodes_inplace(nodes, &sort_state),
            |vm| vm.pid as u32,
            |vm| vm.is_dead = true,
            |vm, col_id, _width| {
                if let Some(f) = vm.fields.iter_mut().find(|f| f.id == *col_id) {
                    if f.numeric >= 0.0 {
                        f.numeric = (f.numeric * 10.0).round() / 10.0;
                    }
                }
            },
        );
        Ok(())
    }

    pub fn get_header_columns(&self) -> Vec<FieldDefDto> {
        self.header_columns.clone()
    }

    pub fn selected_name_for_pid(&self, pid: u32) -> Option<SharedString> {
        self.view
            .flow
            .items()
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| p.name.clone())
    }

    pub fn clear_selection(&mut self) {
        self.view.flow.clear_selection();
    }

    pub fn sort_state(&self) -> &SortState<SharedString> {
        &self.view.flow.sort
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

    pub fn column_metadata(&self) -> Vec<FieldMetadata> {
        self.settings
            .columns()
            .column_metadata()
            .get()
            .clone()
            .into_iter()
            .map(|(k, v)| FieldMetadata {
                is_text: v.is_text,
                is_metric: v.is_metric,
                id: k.into(),
            })
            .collect()
    }
    pub fn column_widths(&self) -> Vec<(SharedString, u64)> {
        self.view
            .layout
            .widths
            .iter()
            .map(|(key, signal)| (key.clone(), signal.get()))
            .collect()
    }

    pub fn toggle_sort(&mut self, field_id: SharedString) {
        let current = &mut self.view.flow.sort;
        if current.field_id.as_ref() == Some(&field_id) {
            current.descending = !current.descending;
        } else {
            current.field_id = Some(field_id);
            current.descending = true;
        }
    }

    pub fn toggle_expand(&mut self, group_id: SharedString) {
        self.view.flow.toggle_expand(group_id);
    }

    pub fn select(&mut self, pid: u32, idx: usize) {
        self.view.flow.select(pid, idx);
    }

    pub fn set_viewport(&mut self, start: usize, count: usize) {
        self.view.rows.set_viewport(start, count);
    }

    pub fn batch(&self) -> TableBatch<'_, ProcessEntryVm> {
        self.view.rows.batch()
    }

    pub fn total_count(&self) -> usize {
        self.view.rows.items.len()
    }
}

fn sort_nodes_inplace(
    nodes: &mut [TableNode<ProcessEntryVm, SharedString>],
    sort: &SortState<SharedString>,
) {
    nodes.sort_by(|a, b| {
        if let Some(ref field_id) = sort.field_id {
            let val_a =
                a.vm.fields
                    .iter()
                    .find(|f| &f.id == field_id)
                    .map(|f| f.numeric)
                    .unwrap_or(-1.0);
            let val_b =
                b.vm.fields
                    .iter()
                    .find(|f| &f.id == field_id)
                    .map(|f| f.numeric)
                    .unwrap_or(-1.0);

            let cmp = val_a
                .partial_cmp(&val_b)
                .unwrap_or(std::cmp::Ordering::Equal);
            if cmp != std::cmp::Ordering::Equal {
                return if sort.descending { cmp.reverse() } else { cmp };
            }
        }
        a.vm.name.cmp(&b.vm.name).then(a.vm.pid.cmp(&b.vm.pid))
    });

    for node in nodes {
        if !node.children.is_empty() {
            sort_nodes_inplace(&mut node.children, sort);
        }
    }
}
