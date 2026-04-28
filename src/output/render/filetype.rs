use nu_ansi_term::AnsiString;

use crate::fs::fields as f;
use crate::theme::Theme;

impl f::Type {
    pub fn render(self, theme: &Theme) -> AnsiString<'static> {
        let kinds = &theme.ui.filekinds;
        match self {
            Self::File => kinds.normal.paint("."),
            Self::Directory => kinds.directory.paint("d"),
            Self::Pipe => kinds.pipe.paint("|"),
            Self::Link => kinds.symlink.paint("l"),
            Self::BlockDevice => kinds.block_device.paint("b"),
            Self::CharDevice => kinds.char_device.paint("c"),
            Self::Socket => kinds.socket.paint("s"),
            Self::Special => kinds.special.paint("?"),
        }
    }
}
