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

pub(crate) fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    let mut parts = full.rsplitn(3, "::");
    match (parts.next(), parts.next()) {
        (Some(name), Some(ns)) => {
            let ns_start = full.len() - ns.len() - name.len() - "::".len();
            &full[ns_start..]
        }
        (Some(name), None) => name,
        _ => full,
    }
}
