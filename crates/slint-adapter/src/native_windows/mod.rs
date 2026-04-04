use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeWindowTexture {
    None,
    Mica,
    Acrylic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeWindowConfig {
    pub texture: NativeWindowTexture,
    pub rounded_corners: bool,
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
    component: Rc<T>,
    config: NativeWindowConfig,
}

impl<T: ComponentHandle> Clone for NativeWindowManager<T> {
    fn clone(&self) -> Self {
        Self {
            component: self.component.clone(),
            config: self.config,
        }
    }
}

impl<T: ComponentHandle + 'static> NativeWindowManager<T> {
    pub fn new(component: Rc<T>) -> Self {
        Self::with_config(component, NativeWindowConfig::default())
    }

    pub fn with_config(component: Rc<T>, config: NativeWindowConfig) -> Self {
        Self { component, config }
    }

    pub fn component(&self) -> Rc<T> {
        self.component.clone()
    }

    pub fn show(&self) -> Result<(), slint::PlatformError> {
        self.component.show()
    }

    pub fn hide(&self) -> Result<(), slint::PlatformError> {
        self.component.hide()
    }

    pub fn drag_window(&self) {
        self.component.window().with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    pub fn apply_effects(&self) {
        apply_to_component(self.component.as_weak(), self.config);
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
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[path = "linux.rs"]
mod platform;
