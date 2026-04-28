use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::theme::Theme;

impl f::VcsRepoStatus {
    pub fn render(&self, theme: &Theme) -> TextCell {
        let vcs = &theme.ui.vcs;
        match self {
            Self::None => TextCell::paint(theme.ui.punctuation, "-".to_string()),
            Self::Repo {
                backend,
                clean,
                branch,
            } => {
                let is_jj = *backend == "jj";
                let indicator = if is_jj {
                    "J"
                } else if *backend == "git" {
                    "G"
                } else {
                    "?"
                };
                let status = if is_jj {
                    // jj uses the green "new" colour as a neutral indicator.
                    vcs.new
                } else if *clean {
                    vcs.new
                } else {
                    vcs.modified
                };

                if let Some(name) = branch {
                    TextCell::paint(status, format!("{indicator} {name}"))
                } else {
                    TextCell::paint(status, indicator.to_string())
                }
            }
        }
    }
}
