use std::io::{self, Write};

use term_grid as tg;

use crate::fs::File;
use crate::fs::filter::FileFilter;
use crate::output::file_name::Options as FileStyle;
use crate::theme::Theme;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub struct Options {
    pub across: bool,
}

impl Options {
    pub fn direction(self) -> tg::Direction {
        if self.across {
            tg::Direction::LeftToRight
        } else {
            tg::Direction::TopToBottom
        }
    }
}

pub struct Render<'a, 'dir> {
    pub files: Vec<File<'dir>>,
    pub theme: &'a Theme,
    pub file_style: &'a FileStyle,
    pub opts: &'a Options,
    pub console_width: usize,
    pub filter: &'a FileFilter,
}

impl Render<'_, '_> {
    pub fn render<W: Write>(mut self, w: &mut W) -> io::Result<()> {
        self.filter.sort_files(&mut self.files, None);

        let cells: Vec<String> = self
            .files
            .iter()
            .map(|file| {
                let filename = self.file_style.for_file(file, self.theme).paint();
                filename.strings().to_string()
            })
            .collect();

        if cells.is_empty() {
            return Ok(());
        }

        let grid = tg::Grid::new(
            cells,
            tg::GridOptions {
                direction: self.opts.direction(),
                filling: tg::Filling::Spaces(2),
                width: self.console_width,
            },
        );

        // If the grid has as many rows as cells, it couldn't fit into
        // multiple columns, so fall back to listing one per line.
        if grid.row_count() >= self.files.len() {
            for file in &self.files {
                let name_cell = self.file_style.for_file(file, self.theme).paint();
                writeln!(w, "{}", name_cell.strings())?;
            }
            Ok(())
        } else {
            write!(w, "{grid}")
        }
    }
}
