use std::time::SystemTime;

use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;
use crate::theme::DateAge;


pub trait Render {
    fn render(self, date_styles: &DateAge, format: &TimeFormat) -> TextCell;
}

impl Render for Option<SystemTime> {
    fn render(self, date_styles: &DateAge, format: &TimeFormat) -> TextCell {
        let (datestamp, style) = if let Some(time) = self {
            let age_secs = SystemTime::now()
                .duration_since(time)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            (format.format(time), date_styles.for_age(age_secs))
        } else {
            (String::from("-"), date_styles.old)
        };

        TextCell::paint(style, datestamp)
    }
}
