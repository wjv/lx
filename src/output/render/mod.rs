mod blocks;

mod filetype;

mod vcs;

mod vcs_repos;

#[cfg(unix)]
mod groups;

mod inode;
// inode uses just one colour

mod links;

mod permissions;

mod size;

mod times;
pub use self::times::Render as TimeRender;
// times does too

#[cfg(unix)]
mod users;

mod octal;
// octal uses just one colour

mod flags;
// flags uses just one colour
