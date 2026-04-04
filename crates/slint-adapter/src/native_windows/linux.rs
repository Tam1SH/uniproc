use super::NativeWindowConfig;
use slint::ComponentHandle;

pub(crate) fn apply_to_component<T: ComponentHandle + 'static>(
    _component: slint::Weak<T>,
    _config: NativeWindowConfig,
) {
}
