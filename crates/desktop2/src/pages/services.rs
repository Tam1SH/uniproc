use guinea::router::{Page, PageCx};
use windows_reactor::{Element, text_block};

pub struct Services;

impl Page for Services {
    fn view(_cx: &mut PageCx) -> Element {
        text_block("services - not ported yet").into()
    }
}
