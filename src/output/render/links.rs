use locale::Numeric as NumericLocale;

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::theme::Theme;

impl f::Links {
    pub fn render(&self, theme: &Theme, numeric: &NumericLocale) -> TextCell {
        let style = if self.multiple {
            theme.ui.links.multi_link_file
        } else {
            theme.ui.links.normal
        };

        TextCell::paint(style, numeric.format_int(self.count))
    }
}

#[cfg(test)]
pub mod test {
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};
    use crate::theme::Theme;

    use nu_ansi_term::Color::*;

    fn theme() -> Theme {
        let mut t = Theme::test_default();
        t.ui.links.normal = Blue.normal();
        t.ui.links.multi_link_file = Blue.on(Red);
        t
    }

    #[test]
    fn regular_file() {
        let stati = f::Links {
            count: 1,
            multiple: false,
        };

        let expected = TextCell {
            width: DisplayWidth::from(1),
            contents: vec![Blue.paint("1")].into(),
        };

        assert_eq!(
            expected,
            stati.render(&theme(), &locale::Numeric::english())
        );
    }

    #[test]
    fn regular_directory() {
        let stati = f::Links {
            count: 3005,
            multiple: false,
        };

        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Blue.paint("3,005")].into(),
        };

        assert_eq!(
            expected,
            stati.render(&theme(), &locale::Numeric::english())
        );
    }

    #[test]
    fn popular_file() {
        let stati = f::Links {
            count: 3005,
            multiple: true,
        };

        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Blue.on(Red).paint("3,005")].into(),
        };

        assert_eq!(
            expected,
            stati.render(&theme(), &locale::Numeric::english())
        );
    }
}
