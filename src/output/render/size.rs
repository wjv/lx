use locale::Numeric as NumericLocale;

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::output::table::SizeFormat;
use crate::theme::Theme;

impl f::Size {
    pub fn render(
        self,
        theme: &Theme,
        size_format: SizeFormat,
        numerics: &NumericLocale,
    ) -> TextCell {
        use unit_prefix::NumberPrefix;

        let size = match self {
            Self::Some(s) => s,
            Self::None => return TextCell::blank(theme.ui.punctuation),
            Self::DeviceIDs(ref ids) => return ids.render(theme),
        };

        let result = match size_format {
            SizeFormat::DecimalBytes => NumberPrefix::decimal(size as f64),
            SizeFormat::BinaryBytes => NumberPrefix::binary(size as f64),
            SizeFormat::JustBytes => {
                // Use the binary prefix to select a style.
                let prefix = match NumberPrefix::binary(size as f64) {
                    NumberPrefix::Standalone(_) => None,
                    NumberPrefix::Prefixed(p, _) => Some(p),
                };

                // But format the number directly using the locale.
                let string = numerics.format_int(size);

                return TextCell::paint(theme.size_style(size, prefix), string);
            }
        };

        let (prefix, n) = match result {
            NumberPrefix::Standalone(b) => {
                return TextCell::paint(theme.size_style(size, None), numerics.format_int(b));
            }
            NumberPrefix::Prefixed(p, n) => (p, n),
        };

        let symbol = prefix.symbol();
        let number = if n < 10_f64 {
            numerics.format_float(n, 1)
        } else {
            numerics.format_int(n.round() as isize)
        };

        TextCell {
            // symbol is guaranteed to be ASCII since unit prefixes are hardcoded.
            width: DisplayWidth::from(&*number) + symbol.len(),
            contents: vec![
                theme.size_style(size, Some(prefix)).paint(number),
                theme.unit_style(Some(prefix)).paint(symbol),
            ]
            .into(),
        }
    }
}

impl f::DeviceIDs {
    fn render(self, theme: &Theme) -> TextCell {
        let major = self.major.to_string();
        let minor = self.minor.to_string();

        TextCell {
            width: DisplayWidth::from(major.len() + 1 + minor.len()),
            contents: vec![
                theme.ui.size.major.paint(major),
                theme.ui.punctuation.paint(","),
                theme.ui.size.minor.paint(minor),
            ]
            .into(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};
    use crate::output::table::SizeFormat;
    use crate::theme::Theme;

    use locale::Numeric as NumericLocale;
    use nu_ansi_term::Color::*;

    fn theme() -> Theme {
        let mut t = Theme::test_default();
        // size_style() returns number_byte for None, number_kilo for
        // Kilo/Kibi, and so on; with all five tiers set to Fixed(66)
        // we mirror the previous "TestColours::size always returns
        // Fixed(66)" behaviour.
        let size_style = Fixed(66).normal();
        t.ui.size.number_byte = size_style;
        t.ui.size.number_kilo = size_style;
        t.ui.size.number_mega = size_style;
        t.ui.size.number_giga = size_style;
        t.ui.size.number_huge = size_style;
        let unit_style = Fixed(77).bold();
        t.ui.size.unit_byte = unit_style;
        t.ui.size.unit_kilo = unit_style;
        t.ui.size.unit_mega = unit_style;
        t.ui.size.unit_giga = unit_style;
        t.ui.size.unit_huge = unit_style;
        t.ui.punctuation = Black.italic();
        t.ui.size.major = Blue.on(Red);
        t.ui.size.minor = Cyan.on(Yellow);
        t
    }

    fn theme_with_punct(punct: nu_ansi_term::Style) -> Theme {
        let mut t = theme();
        t.ui.punctuation = punct;
        t
    }

    #[test]
    fn directory() {
        let directory = f::Size::None;
        let expected = TextCell::blank(Black.italic());
        assert_eq!(
            expected,
            directory.render(&theme(), SizeFormat::JustBytes, &NumericLocale::english())
        );
    }

    #[test]
    fn file_decimal() {
        let directory = f::Size::Some(2_100_000);
        let expected = TextCell {
            width: DisplayWidth::from(4),
            contents: vec![Fixed(66).paint("2.1"), Fixed(77).bold().paint("M")].into(),
        };

        assert_eq!(
            expected,
            directory.render(
                &theme(),
                SizeFormat::DecimalBytes,
                &NumericLocale::english()
            )
        );
    }

    #[test]
    fn file_binary() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![Fixed(66).paint("1.0"), Fixed(77).bold().paint("Mi")].into(),
        };

        assert_eq!(
            expected,
            directory.render(&theme(), SizeFormat::BinaryBytes, &NumericLocale::english())
        );
    }

    #[test]
    fn file_bytes() {
        let directory = f::Size::Some(1_048_576);
        let expected = TextCell {
            width: DisplayWidth::from(9),
            contents: vec![Fixed(66).paint("1,048,576")].into(),
        };

        assert_eq!(
            expected,
            directory.render(&theme(), SizeFormat::JustBytes, &NumericLocale::english())
        );
    }

    #[test]
    fn device_ids() {
        // The comma between major/minor is `theme.ui.punctuation`,
        // which the original test asserted as `Green.italic()`.
        let t = theme_with_punct(Green.italic());
        let directory = f::Size::DeviceIDs(f::DeviceIDs {
            major: 10,
            minor: 80,
        });
        let expected = TextCell {
            width: DisplayWidth::from(5),
            contents: vec![
                Blue.on(Red).paint("10"),
                Green.italic().paint(","),
                Cyan.on(Yellow).paint("80"),
            ]
            .into(),
        };

        assert_eq!(
            expected,
            directory.render(&t, SizeFormat::JustBytes, &NumericLocale::english())
        );
    }
}
