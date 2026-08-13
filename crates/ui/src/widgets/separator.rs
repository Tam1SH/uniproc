use windows_reactor::{border, tokens, Element, ElementExt};

pub fn separator() -> Element {
    border(Element::Empty)
        .height(1.0)
        .background(tokens::DividerStroke)
        .into()
}
