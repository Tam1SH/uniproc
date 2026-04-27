use once_cell::sync::Lazy;
use std::sync::RwLock;

pub mod addr;
pub mod ctx;
pub mod envelope;
pub mod traits;

pub use addr::*;
pub use binder::*;
pub use ctx::*;
pub use envelope::*;
pub use traits::*;

pub mod event_bus;

pub mod binder;
mod macros;
#[cfg(feature = "test-utils")]
pub mod registry;

pub type UiTask = Box<dyn FnOnce() + Send>;

pub trait UiDispatcher: Send + Sync {
    fn init(&self);
    fn dispatch(&self, task: UiTask);
}

static UI_DISPATCHER: Lazy<RwLock<Option<Box<dyn UiDispatcher>>>> = Lazy::new(|| RwLock::new(None));

pub fn set_ui_dispatcher(dispatcher: impl UiDispatcher + 'static) {
    *UI_DISPATCHER.write().unwrap() = Some(Box::new(dispatcher));
}

#[derive(Clone)]
pub struct UiThreadToken(std::marker::PhantomData<*const ()>);

impl UiThreadToken {
    pub unsafe fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

pub(crate) fn invoke_on_ui<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(feature = "test-utils")]
    {
        crate::actor::event_bus::EventBus::queue_test_task(Box::new(f));
    }

    #[cfg(not(feature = "test-utils"))]
    {
        if let Some(dispatcher) = UI_DISPATCHER.read().unwrap().as_ref() {
            dispatcher.dispatch(Box::new(f));
        } else {
            panic!(
                "UiDispatcher not initialized! Call app_core::actor::set_ui_dispatcher at startup."
            );
        }
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
