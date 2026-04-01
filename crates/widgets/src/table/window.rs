pub struct TableBatch<'a, VM> {
    pub total_rows: usize,
    pub start: usize,
    pub rows: &'a [VM],
}

pub struct WindowedRows<VM> {
    pub items: Vec<VM>,
    start: usize,
    count: usize,
}

impl<VM> WindowedRows<VM> {
    pub fn new(default_count: usize) -> Self {
        Self {
            items: Vec::with_capacity(1024),
            start: 0,
            count: default_count.max(1),
        }
    }

    pub fn set_viewport(&mut self, start: usize, count: usize) {
        self.start = start;
        self.count = count.max(1);
    }

    pub fn batch(&self) -> TableBatch<'_, VM> {
        let total = self.items.len();
        let s = self.start.min(total.saturating_sub(1));
        let c = self.count.min(total - s);
        TableBatch {
            total_rows: total,
            start: s,
            rows: if total == 0 {
                &[]
            } else {
                &self.items[s..s + c]
            },
        }
    }
}
