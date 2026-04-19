#![allow(unused)]

use context::native_windows::slint_factory::SlintWindowRegistry;

include!(concat!(env!("OUT_DIR"), "/generated_stubs.rs"));

impl ServicesWindowRegister for ServicesUiStub {
    fn register(&self, registry: &SlintWindowRegistry) {}
}
