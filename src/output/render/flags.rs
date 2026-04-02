use nu_ansi_term::Style;

use crate::fs::fields as f;
use crate::output::cell::TextCell;


impl f::FileFlags {
    pub fn render(self, style: Style) -> TextCell {
        TextCell::paint(style, self.to_short_string())
    }
}
