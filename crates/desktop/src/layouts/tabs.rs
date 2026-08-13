use guinea::router::{Layout, LayoutCx};
use windows_reactor::Element;

pub struct TabsLayout;

impl Layout for TabsLayout {
    fn view(cx: &mut LayoutCx) -> Element {
        cx.outlet()
    }
}
