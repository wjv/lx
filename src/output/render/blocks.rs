use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::theme::Theme;

impl f::Blocks {
    pub fn render(&self, theme: &Theme) -> TextCell {
        match self {
            Self::Some(blk) => TextCell::paint(theme.ui.blocks, blk.to_string()),
            Self::None => TextCell::blank(theme.ui.punctuation),
        }
    }
}

#[cfg(test)]
pub mod test {
    use nu_ansi_term::Color::*;

    use crate::fs::fields as f;
    use crate::output::cell::TextCell;
    use crate::theme::Theme;

    fn theme() -> Theme {
        let mut t = Theme::test_default();
        t.ui.blocks = Red.blink();
        t.ui.punctuation = Green.italic();
        t
    }

    #[test]
    fn blocklessness() {
        let blox = f::Blocks::None;
        let expected = TextCell::blank(Green.italic());
        assert_eq!(expected, blox.render(&theme()));
    }

    #[test]
    fn blockfulity() {
        let blox = f::Blocks::Some(3005);
        let expected = TextCell::paint_str(Red.blink(), "3005");
        assert_eq!(expected, blox.render(&theme()));
    }
}
