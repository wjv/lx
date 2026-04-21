use nu_ansi_term::{AnsiString, Style};

use crate::fs::fields as f;
use crate::output::cell::{DisplayWidth, TextCell};

impl f::VcsFileStatus {
    pub fn render(self, colours: &dyn Colours, backend: &str) -> TextCell {
        if self.staged == self.unstaged {
            // Single-column display (jj, or git with identical status).
            TextCell {
                width: DisplayWidth::from(2),
                contents: vec![
                    self.unstaged.render(colours, backend),
                    colours.not_modified().paint(" "),
                ]
                .into(),
            }
        } else {
            // Two-column display (git staged + unstaged).
            TextCell {
                width: DisplayWidth::from(2),
                contents: vec![
                    self.staged.render(colours, backend),
                    self.unstaged.render(colours, backend),
                ]
                .into(),
            }
        }
    }
}

impl f::VcsStatus {
    fn render(self, colours: &dyn Colours, backend: &str) -> AnsiString<'static> {
        match self {
            Self::NotModified => colours.not_modified().paint("-"),
            Self::New => colours.new().paint(if backend == "JJ" { "A" } else { "N" }),
            Self::Modified => colours.modified().paint("M"),
            Self::Deleted => colours.deleted().paint("D"),
            Self::Renamed => colours.renamed().paint("R"),
            Self::TypeChange => colours.type_change().paint("T"),
            Self::Ignored => colours.ignored().paint("I"),
            Self::Conflicted => colours.conflicted().paint("!"),
            Self::Copied => colours.renamed().paint("C"),
            Self::Untracked => colours.ignored().paint("U"),
        }
    }
}

pub trait Colours {
    fn not_modified(&self) -> Style;
    #[allow(clippy::new_ret_no_self)]
    fn new(&self) -> Style;
    fn modified(&self) -> Style;
    fn deleted(&self) -> Style;
    fn renamed(&self) -> Style;
    fn type_change(&self) -> Style;
    fn ignored(&self) -> Style;
    fn conflicted(&self) -> Style;
}

#[cfg(test)]
pub mod test {
    use super::Colours;
    use crate::fs::fields as f;
    use crate::output::cell::{DisplayWidth, TextCell};

    use nu_ansi_term::Color::*;
    use nu_ansi_term::Style;

    struct TestColours;

    impl Colours for TestColours {
        fn not_modified(&self) -> Style {
            Fixed(90).normal()
        }
        fn new(&self) -> Style {
            Fixed(91).normal()
        }
        fn modified(&self) -> Style {
            Fixed(92).normal()
        }
        fn deleted(&self) -> Style {
            Fixed(93).normal()
        }
        fn renamed(&self) -> Style {
            Fixed(94).normal()
        }
        fn type_change(&self) -> Style {
            Fixed(95).normal()
        }
        fn ignored(&self) -> Style {
            Fixed(96).normal()
        }
        fn conflicted(&self) -> Style {
            Fixed(97).normal()
        }
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

        assert_eq!(expected, stati.render(&TestColours, "Git"))
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

        assert_eq!(expected, stati.render(&TestColours, "Git"))
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

        assert_eq!(expected, stati.render(&TestColours, "JJ"))
    }
}
