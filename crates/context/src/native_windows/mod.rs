use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWindowTexture {
    None,
    Mica,
    MicaAlt,
    Acrylic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWindowConfig {
    pub texture: NativeWindowTexture,
    pub rounded_corners: bool,
}

pub trait ManagedWindowHandle {
    fn show(&self) -> Result<(), slint::PlatformError>;
    fn hide(&self) -> Result<(), slint::PlatformError>;
    fn drag_window(&self);
    fn apply_effects(&self);
    fn focus(&self);
    fn component_any(&self) -> Box<dyn Any>;
    fn cloned(&self) -> Box<dyn ManagedWindowHandle>;
    fn query_interface(&self, type_id: TypeId) -> Option<Box<dyn Any>>;
}

pub trait UiAdapter {
    fn query_port(&self, type_id: TypeId) -> Option<Box<dyn Any>>;
    fn box_clone(&self) -> Box<dyn UiAdapter>;
}

impl dyn ManagedWindowHandle {
    pub fn get_port<P: ?Sized + 'static>(&self) -> Option<Box<P>> {
        self.query_interface(TypeId::of::<P>())
            .and_then(|any| any.downcast::<Box<P>>().ok())
            .map(|boxed| *boxed)
    }
}

impl Debug for dyn ManagedWindowHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedWindow").finish()
    }
}

impl Clone for Box<dyn ManagedWindowHandle> {
    fn clone(&self) -> Box<dyn ManagedWindowHandle> {
        self.cloned()
    }
}

impl Default for NativeWindowConfig {
    fn default() -> Self {
        Self::win11_dialog()
    }
}

impl NativeWindowConfig {
    pub const fn plain() -> Self {
        Self {
            texture: NativeWindowTexture::None,
            rounded_corners: false,
        }
    }

    pub const fn rounded() -> Self {
        Self {
            texture: NativeWindowTexture::None,
            rounded_corners: true,
        }
    }

    pub const fn win11_dialog() -> Self {
        Self {
            texture: NativeWindowTexture::Mica,
            rounded_corners: true,
        }
    }
}

pub struct NativeWindowManager<T: ComponentHandle> {
    component: T,
    config: NativeWindowConfig,
    adapter: Option<Box<dyn UiAdapter>>,
}

impl<T: ComponentHandle> Clone for NativeWindowManager<T> {
    fn clone(&self) -> Self {
        Self {
            component: self.component.clone_strong(),
            config: self.config,
            adapter: self.adapter.as_ref().map(|a| a.box_clone()),
        }
    }
}

impl<T: ComponentHandle> NativeWindowManager<T> {
    pub fn new(component: T) -> Self {
        Self::with_config(component, NativeWindowConfig::default())
    }
    pub fn with_adapter<A: UiAdapter + 'static>(mut self, adapter: A) -> Self {
        self.adapter = Some(adapter.box_clone());
        self
    }
    pub fn with_config(component: T, config: NativeWindowConfig) -> Self {
        Self {
            component,
            config,
            adapter: None,
        }
    }
    pub fn component(&self) -> T {
        self.component.clone_strong()
    }
}

impl<T: ComponentHandle + 'static> ManagedWindowHandle for NativeWindowManager<T> {
    fn show(&self) -> Result<(), slint::PlatformError> {
        self.component.show()
    }
    fn hide(&self) -> Result<(), slint::PlatformError> {
        self.component.hide()
    }

    fn drag_window(&self) {
        self.component.window().with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    fn apply_effects(&self) {
        apply_to_component(self.component.as_weak(), self.config);
    }

    fn focus(&self) {
        self.component
            .window()
            .with_winit_window(|w| w.focus_window());
    }

    fn component_any(&self) -> Box<dyn Any> {
        Box::new(self.component.clone_strong())
    }

    fn cloned(&self) -> Box<dyn ManagedWindowHandle> {
        Box::new(self.clone())
    }

    fn query_interface(&self, type_id: TypeId) -> Option<Box<dyn Any>> {
        self.adapter.as_ref()?.query_port(type_id)
    }
}

pub fn apply_to_component<T: ComponentHandle + 'static>(
    component: slint::Weak<T>,
    config: NativeWindowConfig,
) {
    platform::apply_to_component(component, config);
}

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
pub mod platform;
#[cfg(not(target_os = "windows"))]
#[path = "stub.rs"]
pub mod platform;
pub mod platform_types;
pub mod slint_factory;
