use crate::native_windows::{ManagedWindowHandle, NativeWindowManager};
use app_core::actor::UiThreadToken;
use app_core::actor::event_bus::EventBus;
use app_core::actor::traits::Message;
use app_core::trace::{DispatchMeta, current_meta, is_scope_enabled};
use slint::ComponentHandle;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct OpenWindow {
    pub key: String,
    pub template: String,
    pub data: Arc<dyn Any + Send + Sync>,
}

impl Message for OpenWindow {}

#[derive(Clone)]
pub struct WindowClosed {
    pub key: String,
}
impl Message for WindowClosed {}

type BuilderFn = Box<dyn Fn(&str) -> Box<dyn ManagedWindowHandle>>;

thread_local! {
    static BUILDERS: RefCell<HashMap<String, BuilderFn>> = RefCell::new(HashMap::new());
    static HANDLES: RefCell<HashMap<String, Box<dyn ManagedWindowHandle>>> = RefCell::new(HashMap::new());
}

pub fn get_window(key: &str) -> Option<Box<dyn ManagedWindowHandle>> {
    let handle = HANDLES.with(|h| h.borrow().get(key).map(|handle| handle.cloned()));

    if handle.is_none() {
        if let Some(meta) = current_meta() {
            if is_scope_enabled("context.window.registry") {
                tracing::trace!(
                    parent: &meta.span,
                    key = key,
                    op_id = meta.op_id,
                    correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                    "Window handle not found or dropped"
                );
            }
        }
    }

    handle
}

fn insert_handle(key: &str, handle: Box<dyn ManagedWindowHandle>) {
    if let Some(meta) = current_meta() {
        if is_scope_enabled("context.window.registry") {
            tracing::debug!(
                parent: &meta.span,
                key = key,
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "Inserting window handle"
            );
        }
    }
    HANDLES.with(|h| {
        h.borrow_mut().insert(key.to_string(), handle);
    });
}

fn remove_handle(key: &str) {
    if let Some(meta) = current_meta() {
        if is_scope_enabled("context.window.registry") {
            tracing::debug!(
                parent: &meta.span,
                key = key,
                op_id = meta.op_id,
                correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                "Removing window handle"
            );
        }
    }
    HANDLES.with(|h| {
        h.borrow_mut().remove(key);
    });
}

fn handle_window_close(key: String) {
    remove_handle(&key);
    EventBus::publish(WindowClosed { key });
}

//TODO: in SharedState make it Box with generic trait name.
pub struct SlintWindowRegistry;

impl SlintWindowRegistry {
    pub fn new() -> Self {
        Self
    }
}

pub trait WindowRegistry: Send + Sync + 'static {
    fn get_window(&self, key: &str) -> Option<Box<dyn ManagedWindowHandle>>;
    fn register<T, F>(&self, template_name: &str, builder: F)
    where
        T: ComponentHandle + 'static,
        F: Fn() -> NativeWindowManager<T> + 'static;
    fn build_window(
        &self,
        _guard: &UiThreadToken,
        template: &str,
        key: &str,
    ) -> Option<Box<dyn ManagedWindowHandle>>;
}

impl WindowRegistry for SlintWindowRegistry {
    fn get_window(&self, key: &str) -> Option<Box<dyn ManagedWindowHandle>> {
        get_window(key)
    }

    fn register<T, F>(&self, template_name: &str, builder: F)
    where
        T: ComponentHandle + 'static,
        F: Fn() -> NativeWindowManager<T> + 'static,
    {
        let template_owned = template_name.to_string();

        let builder_fn = move |key: &str| -> Box<dyn ManagedWindowHandle> {
            let meta = current_meta()
                .unwrap_or_else(|| DispatchMeta::capture_or_root("context.window.registry"));

            if is_scope_enabled("context.window.registry") {
                info!(
                    parent: &meta.span,
                    key = key,
                    template = template_owned.clone(),
                    op_id = meta.op_id,
                    correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                    "Building and showing window"
                );
            }

            let mgr = builder();
            mgr.apply_effects();
            mgr.show().unwrap();

            let handle: Box<dyn ManagedWindowHandle> = Box::new(mgr.clone());
            insert_handle(key, handle);

            let key_owned = key.to_string();
            let key_inner = key_owned.to_string();
            let template_owned = template_owned.to_string();

            mgr.component().window().on_close_requested(move || {
                let meta = current_meta()
                    .unwrap_or_else(|| DispatchMeta::capture_or_root("context.window.registry"));

                if is_scope_enabled("context.window.registry") {
                    info!(
                        parent: &meta.span,
                        key = key_inner,
                        template = template_owned.clone(),
                        op_id = meta.op_id,
                        correlation_id = meta.correlation_id.as_deref().unwrap_or(""),
                        "close window"
                    );
                }

                handle_window_close(key_owned.clone());
                slint::CloseRequestResponse::HideWindow
            });

            Box::new(mgr)
        };

        BUILDERS.with(|b| {
            b.borrow_mut()
                .insert(template_name.to_string(), Box::new(builder_fn));
        });
    }

    fn build_window(
        &self,
        _guard: &UiThreadToken,
        template: &str,
        key: &str,
    ) -> Option<Box<dyn ManagedWindowHandle>> {
        let meta =
            current_meta().map(|m| m.child("context.window.registry.build_window", None, None));

        if let Some(ref m) = meta {
            if is_scope_enabled("context.window.registry") {
                tracing::debug!(
                    parent: &m.span,
                    key = key,
                    template = template,
                    op_id = m.op_id,
                    correlation_id = m.correlation_id.as_deref().unwrap_or(""),
                    "Requested window build"
                );
            }
        }

        let result = BUILDERS.with(|b| {
            let builders = b.borrow();
            builders.get(template).map(|f| f(key))
        });

        if result.is_none() {
            if let Some(ref m) = meta {
                if is_scope_enabled("context.window.registry") {
                    tracing::warn!(
                        parent: &m.span,
                        key = key,
                        template = template,
                        op_id = m.op_id,
                        correlation_id = m.correlation_id.as_deref().unwrap_or(""),
                        "No builder found for template"
                    );
                }
            }
        }

        result
    }
}
