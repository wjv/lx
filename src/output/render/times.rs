use std::time::SystemTime;

use nu_ansi_term::Style;

use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;


pub trait Render {
    fn render(self, style: Style, format: &TimeFormat) -> TextCell;
}

impl Render for Option<SystemTime> {
    fn render(self, style: Style, format: &TimeFormat) -> TextCell {
        let datestamp = if let Some(time) = self {
            format.format(time)
        }
        else {
            String::from("-")
        };

        TextCell::paint(style, datestamp)
    }
}
