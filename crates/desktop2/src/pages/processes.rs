use guinea::router::{Page, PageCx};
use windows_reactor::{Element, text_block};

pub struct Processes;

impl Page for Processes {
    fn view(_cx: &mut PageCx) -> Element {
        text_block("processes - not ported yet").into()
    }
}
