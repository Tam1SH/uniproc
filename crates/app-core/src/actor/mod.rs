pub mod addr;
pub mod envelope;
mod macros;
pub mod traits;

pub mod event_bus;

#[derive(Clone)]
pub struct UiThreadGuard(std::marker::PhantomData<*const ()>);

impl UiThreadGuard {
    pub(crate) fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}
