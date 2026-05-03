use nu_ansi_term::{AnsiString, Style};

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::theme::Theme;

impl f::PermissionsPlus {
    #[cfg(unix)]
    pub fn render(&self, theme: &Theme) -> TextCell {
        let mut chars = vec![self.file_type.render(theme)];
        chars.extend(
            self.permissions
                .render(theme, self.file_type.is_regular_file()),
        );

        if self.xattrs {
            chars.push(theme.ui.perms.attribute.paint("@"));
        }

        // As these are all ASCII characters, we can guarantee that they’re
        // all going to be one character wide, and don’t need to compute the
        // cell’s display width.
        TextCell {
            width: DisplayWidth::from(chars.len()),
            contents: chars.into(),
        }
    }

    #[cfg(windows)]
    pub fn render(&self, theme: &Theme) -> TextCell {
        let mut chars = vec![self.attributes.render_type(theme)];
        chars.extend(self.attributes.render(theme));

        TextCell {
            width: DisplayWidth::from(chars.len()),
            contents: chars.into(),
        }
    }
}

impl f::Permissions {
    pub fn render(&self, theme: &Theme, is_regular_file: bool) -> Vec<AnsiString<'static>> {
        let perms = &theme.ui.perms;
        let dash = theme.ui.punctuation;
        let bit = |bit, chr: &'static str, style: Style| {
            if bit {
                style.paint(chr)
            } else {
                dash.paint("-")
            }
        };

        vec![
            bit(self.user_read, "r", perms.user_read),
            bit(self.user_write, "w", perms.user_write),
            self.user_execute_bit(theme, is_regular_file),
            bit(self.group_read, "r", perms.group_read),
            bit(self.group_write, "w", perms.group_write),
            self.group_execute_bit(theme),
            bit(self.other_read, "r", perms.other_read),
            bit(self.other_write, "w", perms.other_write),
            self.other_execute_bit(theme),
        ]
    }

    fn user_execute_bit(&self, theme: &Theme, is_regular_file: bool) -> AnsiString<'static> {
        let perms = &theme.ui.perms;
        let dash = theme.ui.punctuation;
        match (self.user_execute, self.setuid, is_regular_file) {
            (false, false, _) => dash.paint("-"),
            (true, false, false) => perms.user_execute_other.paint("x"),
            (true, false, true) => perms.user_execute_file.paint("x"),
            (false, true, _) => perms.special_other.paint("S"),
            (true, true, false) => perms.special_other.paint("s"),
            (true, true, true) => perms.special_user_file.paint("s"),
        }
    }

    fn group_execute_bit(&self, theme: &Theme) -> AnsiString<'static> {
        let perms = &theme.ui.perms;
        let dash = theme.ui.punctuation;
        match (self.group_execute, self.setgid) {
            (false, false) => dash.paint("-"),
            (true, false) => perms.group_execute.paint("x"),
            (false, true) => perms.special_other.paint("S"),
            (true, true) => perms.special_other.paint("s"),
        }
    }

    fn other_execute_bit(&self, theme: &Theme) -> AnsiString<'static> {
        let perms = &theme.ui.perms;
        let dash = theme.ui.punctuation;
        match (self.other_execute, self.sticky) {
            (false, false) => dash.paint("-"),
            (true, false) => perms.other_execute.paint("x"),
            (false, true) => perms.special_other.paint("T"),
            (true, true) => perms.special_other.paint("t"),
        }
    }
}

#[cfg(windows)]
impl f::Attributes {
    pub fn render(&self, theme: &Theme) -> Vec<AnsiString<'static>> {
        let perms = &theme.ui.perms;
        let dash = theme.ui.punctuation;
        let bit = |bit, chr: &'static str, style: Style| {
            if bit {
                style.paint(chr)
            } else {
                dash.paint("-")
            }
        };

        vec![
            bit(self.archive, "a", theme.ui.filekinds.normal),
            bit(self.readonly, "r", perms.user_read),
            bit(self.hidden, "h", perms.special_user_file),
            bit(self.system, "s", perms.special_other),
        ]
    }

    pub fn render_type(&self, theme: &Theme) -> AnsiString<'static> {
        let kinds = &theme.ui.filekinds;
        if self.reparse_point {
            kinds.pipe.paint("l")
        } else if self.directory {
            kinds.directory.paint("d")
        } else {
            theme.ui.punctuation.paint("-")
        }
    }
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use crate::fs::fields as f;
    use crate::output::cell::TextCellContents;
    use crate::theme::Theme;

    use nu_ansi_term::Color::*;

    fn theme() -> Theme {
        let mut t = Theme::test_default();
        t.ui.punctuation = Fixed(11).normal();
        t.ui.perms.user_read = Fixed(101).normal();
        t.ui.perms.user_write = Fixed(102).normal();
        t.ui.perms.user_execute_file = Fixed(103).normal();
        t.ui.perms.user_execute_other = Fixed(113).normal();
        t.ui.perms.group_read = Fixed(104).normal();
        t.ui.perms.group_write = Fixed(105).normal();
        t.ui.perms.group_execute = Fixed(106).normal();
        t.ui.perms.other_read = Fixed(107).normal();
        t.ui.perms.other_write = Fixed(108).normal();
        t.ui.perms.other_execute = Fixed(109).normal();
        t.ui.perms.special_user_file = Fixed(110).normal();
        t.ui.perms.special_other = Fixed(111).normal();
        t.ui.perms.attribute = Fixed(112).normal();
        t
    }

    #[test]
    fn negate() {
        let bits = f::Permissions {
            user_read: false,
            user_write: false,
            user_execute: false,
            setuid: false,
            group_read: false,
            group_write: false,
            group_execute: false,
            setgid: false,
            other_read: false,
            other_write: false,
            other_execute: false,
            sticky: false,
        };

        let expected = TextCellContents::from(vec![
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
        ]);

        assert_eq!(expected, bits.render(&theme(), false).into());
    }

    #[test]
    fn affirm() {
        let bits = f::Permissions {
            user_read: true,
            user_write: true,
            user_execute: true,
            setuid: false,
            group_read: true,
            group_write: true,
            group_execute: true,
            setgid: false,
            other_read: true,
            other_write: true,
            other_execute: true,
            sticky: false,
        };

        let expected = TextCellContents::from(vec![
            Fixed(101).paint("r"),
            Fixed(102).paint("w"),
            Fixed(103).paint("x"),
            Fixed(104).paint("r"),
            Fixed(105).paint("w"),
            Fixed(106).paint("x"),
            Fixed(107).paint("r"),
            Fixed(108).paint("w"),
            Fixed(109).paint("x"),
        ]);

        assert_eq!(expected, bits.render(&theme(), true).into());
    }

    #[test]
    fn specials() {
        let bits = f::Permissions {
            user_read: false,
            user_write: false,
            user_execute: true,
            setuid: true,
            group_read: false,
            group_write: false,
            group_execute: true,
            setgid: true,
            other_read: false,
            other_write: false,
            other_execute: true,
            sticky: true,
        };

        let expected = TextCellContents::from(vec![
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(110).paint("s"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(111).paint("s"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(111).paint("t"),
        ]);

        assert_eq!(expected, bits.render(&theme(), true).into());
    }

    #[test]
    fn extra_specials() {
        let bits = f::Permissions {
            user_read: false,
            user_write: false,
            user_execute: false,
            setuid: true,
            group_read: false,
            group_write: false,
            group_execute: false,
            setgid: true,
            other_read: false,
            other_write: false,
            other_execute: false,
            sticky: true,
        };

        let expected = TextCellContents::from(vec![
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(111).paint("S"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(111).paint("S"),
            Fixed(11).paint("-"),
            Fixed(11).paint("-"),
            Fixed(111).paint("T"),
        ]);

        assert_eq!(expected, bits.render(&theme(), true).into());
    }
}
