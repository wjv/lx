use nu_ansi_term::Style;

use crate::fs::fields as f;
use crate::output::cell::TextCell;

impl f::VcsRepoStatus {
    pub fn render(&self, colours: &dyn Colours) -> TextCell {
        match self {
            Self::None => TextCell::paint(colours.not_a_repo(), "-".to_string()),
            Self::Repo {
                backend,
                clean,
                branch,
            } => {
                let indicator = match *backend {
                    "jj" => "J",
                    "git" => "G",
                    _ => "?",
                };
                let status = if *backend == "jj" {
                    colours.jj_repo()
                } else if *clean {
                    colours.clean_repo()
                } else {
                    colours.dirty_repo()
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

pub trait Colours {
    fn not_a_repo(&self) -> Style;
    fn clean_repo(&self) -> Style;
    fn dirty_repo(&self) -> Style;
    fn jj_repo(&self) -> Style;
}
