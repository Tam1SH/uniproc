pub struct WindowBatch<T> {
    pub total_rows: usize,
    pub start: usize,
    pub rows: Vec<T>,
}

pub struct WindowedRows<T> {
    items: Vec<T>,
    start: usize,
    count: usize,
}

impl<T> WindowedRows<T> {
    pub fn new(default_count: usize) -> Self {
        Self {
            items: Vec::new(),
            start: 0,
            count: default_count.max(1),
        }
    }

    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
    }

    pub fn set_viewport(&mut self, start: usize, count: usize) {
        self.start = start;
        self.count = count.max(1);
    }

    pub fn total_rows(&self) -> usize {
        self.items.len()
    }

    pub fn range(&self) -> (usize, usize) {
        clamp_window(self.start, self.count, self.items.len())
    }
}

impl<T: Clone> WindowedRows<T> {
    pub fn batch(&self) -> WindowBatch<T> {
        let total_rows = self.items.len();
        let (start, count) = self.range();
        let rows = if count == 0 {
            Vec::new()
        } else {
            self.items[start..start + count].to_vec()
        };
        WindowBatch {
            total_rows,
            start,
            rows,
        }
    }
}

fn clamp_window(start: usize, count: usize, total: usize) -> (usize, usize) {
    if total == 0 || count == 0 {
        return (0, 0);
    }
    let s = start.min(total.saturating_sub(1));
    let c = count.min(total - s);
    (s, c)
}

#[cfg(test)]
mod tests {
    use super::WindowedRows;

    #[test]
    fn batch_is_clamped_to_total() {
        let mut rows = WindowedRows::new(10);
        rows.set_items(vec![1, 2, 3, 4]);
        rows.set_viewport(3, 10);

        let b = rows.batch();
        assert_eq!(b.total_rows, 4);
        assert_eq!(b.start, 3);
        assert_eq!(b.rows, vec![4]);
    }
}
