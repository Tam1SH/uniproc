use crate::table::flow::{SortState, TableDataBuilder, TableFlowState, TableNode};
use crate::table::layout::TableLayout;
use crate::table::window::WindowedRows;
use std::hash::Hash;

pub struct TableView<T, VM, ID, GID, SID, COLID> {
    pub flow: TableFlowState<T, VM, ID, GID, SID>,
    pub layout: TableLayout<COLID>,
    pub rows: WindowedRows<VM>,
}

impl<T, VM, ID, GID, SID, COLID> TableView<T, VM, ID, GID, SID, COLID>
where
    VM: Clone,
    ID: PartialEq + Clone,
    GID: Eq + Hash + Clone,
    SID: Clone,
    COLID: Hash + Eq + Clone + Send + Sync + 'static,
{
    pub fn new(default_sort: SortState<SID>, viewport_size: usize) -> Self {
        Self {
            flow: TableFlowState::new(default_sort),
            layout: TableLayout::new(),
            rows: WindowedRows::new(viewport_size),
        }
    }

    pub fn refresh_full(
        &mut self,
        builder: &mut impl TableDataBuilder<T, VM, GID>,
        sorter: impl Fn(&mut [TableNode<VM, GID>], &SortState<SID>),
        get_id: impl Fn(&VM) -> ID,
        mark_dead: impl Fn(&mut VM),
        mut patch_fn: impl FnMut(&mut VM, &COLID, u64),
    ) {
        self.flow
            .update_view_model(&mut self.rows.items, builder, sorter, get_id, mark_dead);

        let widths_snapshot = self.layout.snapshot();

        for vm in &mut self.rows.items {
            for (col_id, width) in &widths_snapshot {
                patch_fn(vm, col_id, *width);
            }
        }
    }

    pub fn patch_column_width(
        &mut self,
        col_id: &COLID,
        new_width: u64,
        mut patch_fn: impl FnMut(&mut VM, u64),
    ) {
        self.layout.set_width(col_id, new_width);

        for vm in &mut self.rows.items {
            patch_fn(vm, new_width);
        }
    }
}
