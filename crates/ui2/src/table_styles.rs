use windows_reactor::{text_block, Color, Element, ElementExt};

#[derive(Clone, Copy, Debug)]
pub struct TableStyles {
    pub row_height: f64,
    pub font_size: f64,
    pub separator_color: Color,
    pub terminate_icon_size: f64,
}

impl TableStyles {
    //const Default?, lol.
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self {
            row_height: 16.0,
            font_size: 12.0,
            separator_color: Color {
                a: 48,
                r: 128,
                g: 128,
                b: 128,
            },
            terminate_icon_size: 16.0,
        }
    }

    pub fn text_cell(&self, content: impl Into<String>) -> Element {
        text_block(content)
            .font_size(self.font_size)
            .height(self.row_height)
            .max_height(self.row_height)
            .into()
    }
}

pub const TABLE_STYLES: TableStyles = TableStyles::new();
