use std::ffi::OsString;

// General variables

/// Environment variable used to colour files, both by their filesystem type
/// (symlink, socket, directory) and their file name or extension (image,
/// video, archive);
pub static LS_COLORS: &str = "LS_COLORS";

/// Environment variable used to override the width of the terminal, in
/// characters.
pub static COLUMNS: &str = "COLUMNS";

/// Environment variable used to datetime format.
pub static TIME_STYLE: &str = "TIME_STYLE";

/// Environment variable used to disable colours.
/// See: <https://no-color.org/>
pub static NO_COLOR: &str = "NO_COLOR";

// lx-specific variables

/// Environment variable used to make lx print out debugging information as
/// it runs. Any non-empty value will turn debug mode on.
pub static LX_DEBUG: &str = "LX_DEBUG";

/// Environment variable used to limit the grid-details view
/// (`--grid --long`) so it's only activated if there's at least the given
/// number of rows of output.
pub static LX_GRID_ROWS: &str = "LX_GRID_ROWS";

/// Environment variable used to specify how many spaces to print between an
/// icon and its file name. Different terminals display icons differently,
/// with 1 space bringing them too close together or 2 spaces putting them too
/// far apart, so this may be necessary depending on how they are shown.
pub static LX_ICON_SPACING: &str = "LX_ICON_SPACING";

/// Mockable wrapper for `std::env::var_os`.
pub trait Vars {
    fn get(&self, name: &'static str) -> Option<OsString>;
}

// Test impl that just returns the value it has.
#[cfg(test)]
impl Vars for Option<OsString> {
    fn get(&self, _name: &'static str) -> Option<OsString> {
        self.clone()
    }
}
