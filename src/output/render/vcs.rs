use nu_ansi_term::AnsiString;

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};
use crate::theme::Theme;

impl f::VcsFileStatus {
    pub fn render(self, theme: &Theme, backend: &str) -> TextCell {
        if self.staged == self.unstaged {
            // Single-column display (jj, or git with identical status).
            TextCell {
                width: DisplayWidth::from(2),
                contents: vec![
                    self.unstaged.render(theme, backend),
                    theme.ui.punctuation.paint(" "),
                ]
                .into(),
            }
        } else {
            // Two-column display (git staged + unstaged).
            TextCell {
                width: DisplayWidth::from(2),
                contents: vec![
                    self.staged.render(theme, backend),
                    self.unstaged.render(theme, backend),
                ]
                .into(),
            }
        }
    }
}

impl f::VcsStatus {
    fn render(self, theme: &Theme, backend: &str) -> AnsiString<'static> {
        let vcs = &theme.ui.vcs;
        match self {
            Self::NotModified => theme.ui.punctuation.paint("-"),
            Self::New => vcs.new.paint(if backend == "JJ" { "A" } else { "N" }),
            Self::Modified => vcs.modified.paint("M"),
            Self::Deleted => vcs.deleted.paint("D"),
            Self::Renamed => vcs.renamed.paint("R"),
            Self::TypeChange => vcs.typechange.paint("T"),
            Self::Ignored => vcs.ignored.paint("I"),
            Self::Conflicted => vcs.conflicted.paint("!"),
            Self::Copied => vcs.renamed.paint("C"),
            Self::Untracked => vcs.ignored.paint("U"),
        }
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
        t.ui.punctuation = Fixed(90).normal();
        t.ui.vcs.new = Fixed(91).normal();
        t.ui.vcs.modified = Fixed(92).normal();
        t.ui.vcs.deleted = Fixed(93).normal();
        t.ui.vcs.renamed = Fixed(94).normal();
        t.ui.vcs.typechange = Fixed(95).normal();
        t.ui.vcs.ignored = Fixed(96).normal();
        t.ui.vcs.conflicted = Fixed(97).normal();
        t
    }

    #[test]
    fn vcs_blank() {
        let stati = f::VcsFileStatus {
            staged: f::VcsStatus::NotModified,
            unstaged: f::VcsStatus::NotModified,
        };

        // Equal statuses → single-column display (char + space).
        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(90).paint("-"), Fixed(90).paint(" ")].into(),
        };

        assert_eq!(expected, stati.render(&theme(), "Git"));
    }

    #[test]
    fn vcs_staged_unstaged_differ() {
        let stati = f::VcsFileStatus {
            staged: f::VcsStatus::New,
            unstaged: f::VcsStatus::Modified,
        };

        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(91).paint("N"), Fixed(92).paint("M")].into(),
        };

        assert_eq!(expected, stati.render(&theme(), "Git"));
    }

    #[test]
    fn vcs_jj_new_shows_a() {
        let stati = f::VcsFileStatus {
            staged: f::VcsStatus::New,
            unstaged: f::VcsStatus::New,
        };

        let expected = TextCell {
            width: DisplayWidth::from(2),
            contents: vec![Fixed(91).paint("A"), Fixed(90).paint(" ")].into(),
        };

        assert_eq!(expected, stati.render(&theme(), "JJ"));
    }
}
