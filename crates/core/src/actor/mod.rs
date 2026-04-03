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
    let raw = full.split('<').next().unwrap_or(full);
    let mut parts = raw.rsplitn(3, "::");
    let raw = match (parts.next(), parts.next()) {
        (Some(name), Some(ns)) => {
            let ns_start = raw.len() - ns.len() - name.len() - "::".len();
            &raw[ns_start..]
        }
        (Some(name), None) => name,
        _ => raw,
    };

    raw.trim_end_matches('>')
}

pub(crate) fn should_trace_actor_message(message: &str) -> bool {
    !matches!(message, "agents::ScanTick")
}
