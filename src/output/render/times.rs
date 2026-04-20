use std::time::SystemTime;

use nu_ansi_term::Style;

use crate::output::cell::TextCell;
use crate::output::time::TimeFormat;
use crate::theme::{DateAge, age_to_position};

pub trait Render {
    /// Render a timestamp into a `TextCell`.
    ///
    /// `date_styles` carries the six discrete per-tier colours.
    /// `smooth_lut`, if `Some`, is a reference to the 256-stop
    /// interpolated LUT produced by `apply_gradient_flags`:
    /// when present, it overrides the discrete tier lookup with
    /// a position-based smooth colour.
    fn render(
        self,
        date_styles: &DateAge,
        smooth_lut: Option<&[Style; 256]>,
        format: &TimeFormat,
    ) -> TextCell;
}

impl Render for Option<SystemTime> {
    fn render(
        self,
        date_styles: &DateAge,
        smooth_lut: Option<&[Style; 256]>,
        format: &TimeFormat,
    ) -> TextCell {
        let (datestamp, style) = if let Some(time) = self {
            let age_secs = SystemTime::now()
                .duration_since(time)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let style = if let Some(lut) = smooth_lut {
                let position = age_to_position(age_secs);
                let bucket = (position * 255.0).round() as usize;
                lut[bucket.min(255)]
            } else {
                date_styles.for_age(age_secs)
            };

            (format.format(time), style)
        } else {
            // Missing timestamp: always use the discrete `old`
            // colour, even in smooth mode — the file has no
            // meaningful position on the gradient.
            (String::from("-"), date_styles.old)
        };

        TextCell::paint(style, datestamp)
    }
}
