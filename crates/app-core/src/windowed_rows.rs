pub struct WindowBatch<'a, T> {
    pub total_rows: usize,
    pub start: usize,
    pub rows: &'a [T],
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

    pub fn set_items(&mut self, items: &[T])
    where
        T: Clone,
    {
        self.items.clear();
        self.items.extend_from_slice(items);
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

    pub fn batch(&self) -> WindowBatch<'_, T> {
        let (start, count) = self.range();
        WindowBatch {
            total_rows: self.items.len(),
            start,
            rows: &self.items[start..start + count],
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
        rows.set_items(&*vec![1, 2, 3, 4]);
        rows.set_viewport(3, 10);

        let b = rows.batch();
        assert_eq!(b.total_rows, 4);
        assert_eq!(b.start, 3);
        assert_eq!(b.rows, vec![4]);
    }
}
