#[must_use = "UI Interaction must be stabilized. Call .stabilize(&mut harness) to process events."]
pub struct Interaction<T> {
    value: T,
}

pub trait Stabilizer {
    fn stabilize(&mut self);
}

impl<T> Interaction<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn stabilize(self, harness: &mut impl Stabilizer) -> T {
        harness.stabilize();
        self.value
    }
}
