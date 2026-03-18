use nu_ansi_term::Style;

use crate::fs::File;
use crate::output::file_name::Colours as FileNameColours;
use crate::output::render;

mod ui_styles;
pub use self::ui_styles::UiStyles;

mod lsc;
pub use self::lsc::LSColors;

mod default_theme;


#[derive(PartialEq, Eq, Debug)]
pub struct Options {

    pub use_colours: UseColours,

    pub colour_scale: ColourScale,

    pub definitions: Definitions,
}

/// Under what circumstances we should display coloured, rather than plain,
/// output to the terminal.
///
/// By default, we want to display the colours when stdout can display them.
/// Turning them on when output is going to, say, a pipe, would make programs
/// such as `grep` or `more` not work properly. So the `Automatic` mode does
/// this check and only displays colours when they can be truly appreciated.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum UseColours {

    /// Display them even when output isn’t going to a terminal.
    Always,

    /// Display them when output is going to a terminal, but not otherwise.
    Automatic,

    /// Never display them, even when output is going to a terminal.
    Never,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ColourScale {
    Fixed,
    Gradient,
}

#[derive(PartialEq, Eq, Debug, Default)]
pub struct Definitions {
    pub ls: Option<String>,
    pub lx: Option<String>,
}


pub struct Theme {
    pub ui: UiStyles,
    pub exts: Box<dyn FileColours>,
}

impl Options {

    #[allow(trivial_casts)]   // the `as Box<_>` stuff below warns about this for some reason
    pub fn to_theme(&self, isatty: bool) -> Theme {
        use crate::info::filetype::FileExtensions;

        if self.use_colours == UseColours::Never || (self.use_colours == UseColours::Automatic && ! isatty) {
            let ui = UiStyles::plain();
            let exts = Box::new(NoFileColours);
            return Theme { ui, exts };
        }

        // Parse the environment variables into colours and extension mappings
        let mut ui = UiStyles::default_theme(self.colour_scale);
        let (exts, use_default_filetypes) = self.definitions.parse_color_vars(&mut ui);

        // Use between 0 and 2 file name highlighters
        let exts = match (exts.is_non_empty(), use_default_filetypes) {
            (false, false)  => Box::new(NoFileColours)           as Box<_>,
            (false,  true)  => Box::new(FileExtensions)          as Box<_>,
            ( true, false)  => Box::new(exts)                    as Box<_>,
            ( true,  true)  => Box::new((exts, FileExtensions))  as Box<_>,
        };

        Theme { ui, exts }
    }
}

impl Definitions {

    /// Parse the environment variables into `LS_COLORS` pairs, putting file glob
    /// colours into the `ExtensionMappings` that gets returned, and using the
    /// two-character UI codes to modify the mutable `Colours`.
    ///
    /// Also returns if the `LX_COLORS` variable should reset the existing file
    /// type mappings or not. The `reset` code needs to be the first one.
    fn parse_color_vars(&self, colours: &mut UiStyles) -> (ExtensionMappings, bool) {
        use log::*;

        let mut exts = ExtensionMappings::default();

        if let Some(lsc) = &self.ls {
            LSColors(lsc).each_pair(|pair| {
                if ! colours.set_ls(&pair) {
                    match glob::Pattern::new(pair.key) {
                        Ok(pat) => {
                            exts.add(pat, pair.to_style());
                        }
                        Err(e) => {
                            warn!("Couldn't parse glob pattern {:?}: {}", pair.key, e);
                        }
                    }
                }
            });
        }

        let mut use_default_filetypes = true;

        if let Some(lx) = &self.lx {
            // Is this hacky? Yes.
            if lx == "reset" || lx.starts_with("reset:") {
                use_default_filetypes = false;
            }

            LSColors(lx).each_pair(|pair| {
                if ! colours.set_ls(&pair) && ! colours.set_lx(&pair) {
                    match glob::Pattern::new(pair.key) {
                        Ok(pat) => {
                            exts.add(pat, pair.to_style());
                        }
                        Err(e) => {
                            warn!("Couldn't parse glob pattern {:?}: {}", pair.key, e);
                        }
                    }
                }
            });
        }

        (exts, use_default_filetypes)
    }
}


pub trait FileColours: std::marker::Sync {
    fn colour_file(&self, file: &File<'_>) -> Option<Style>;
}

#[derive(PartialEq, Debug)]
struct NoFileColours;
impl FileColours for NoFileColours {
    fn colour_file(&self, _file: &File<'_>) -> Option<Style> {
        None
    }
}

// When getting the colour of a file from a *pair* of colourisers, try the
// first one then try the second one. This lets the user provide their own
// file type associations, while falling back to the default set if not set
// explicitly.
impl<A, B> FileColours for (A, B)
where A: FileColours,
      B: FileColours,
{
    fn colour_file(&self, file: &File<'_>) -> Option<Style> {
        self.0.colour_file(file)
            .or_else(|| self.1.colour_file(file))
    }
}


#[derive(PartialEq, Debug, Default)]
struct ExtensionMappings {
    mappings: Vec<(glob::Pattern, Style)>,
}

// Loop through backwards so that colours specified later in the list override
// colours specified earlier, like we do with options and strict mode

impl FileColours for ExtensionMappings {
    fn colour_file(&self, file: &File<'_>) -> Option<Style> {
        self.mappings.iter().rev()
            .find(|t| t.0.matches(&file.name))
            .map (|t| t.1)
    }
}

impl ExtensionMappings {
    fn is_non_empty(&self) -> bool {
        ! self.mappings.is_empty()
    }

    fn add(&mut self, pattern: glob::Pattern, style: Style) {
        self.mappings.push((pattern, style));
    }
}




impl render::BlocksColours for Theme {
    fn block_count(&self)  -> Style { self.ui.blocks }
    fn no_blocks(&self)    -> Style { self.ui.punctuation }
}

impl render::FiletypeColours for Theme {
    fn normal(&self)       -> Style { self.ui.filekinds.normal }
    fn directory(&self)    -> Style { self.ui.filekinds.directory }
    fn pipe(&self)         -> Style { self.ui.filekinds.pipe }
    fn symlink(&self)      -> Style { self.ui.filekinds.symlink }
    fn block_device(&self) -> Style { self.ui.filekinds.block_device }
    fn char_device(&self)  -> Style { self.ui.filekinds.char_device }
    fn socket(&self)       -> Style { self.ui.filekinds.socket }
    fn special(&self)      -> Style { self.ui.filekinds.special }
}

impl render::VcsColours for Theme {
    fn not_modified(&self)  -> Style { self.ui.punctuation }
    #[allow(clippy::new_ret_no_self)]
    fn new(&self)           -> Style { self.ui.vcs.new }
    fn modified(&self)      -> Style { self.ui.vcs.modified }
    fn deleted(&self)       -> Style { self.ui.vcs.deleted }
    fn renamed(&self)       -> Style { self.ui.vcs.renamed }
    fn type_change(&self)   -> Style { self.ui.vcs.typechange }
    fn ignored(&self)       -> Style { self.ui.vcs.ignored }
    fn conflicted(&self)    -> Style { self.ui.vcs.conflicted }
}

#[cfg(unix)]
impl render::GroupColours for Theme {
    fn yours(&self)      -> Style { self.ui.users.group_yours }
    fn not_yours(&self)  -> Style { self.ui.users.group_not_yours }
}

impl render::LinksColours for Theme {
    fn normal(&self)           -> Style { self.ui.links.normal }
    fn multi_link_file(&self)  -> Style { self.ui.links.multi_link_file }
}

impl render::PermissionsColours for Theme {
    fn dash(&self)               -> Style { self.ui.punctuation }
    fn user_read(&self)          -> Style { self.ui.perms.user_read }
    fn user_write(&self)         -> Style { self.ui.perms.user_write }
    fn user_execute_file(&self)  -> Style { self.ui.perms.user_execute_file }
    fn user_execute_other(&self) -> Style { self.ui.perms.user_execute_other }
    fn group_read(&self)         -> Style { self.ui.perms.group_read }
    fn group_write(&self)        -> Style { self.ui.perms.group_write }
    fn group_execute(&self)      -> Style { self.ui.perms.group_execute }
    fn other_read(&self)         -> Style { self.ui.perms.other_read }
    fn other_write(&self)        -> Style { self.ui.perms.other_write }
    fn other_execute(&self)      -> Style { self.ui.perms.other_execute }
    fn special_user_file(&self)  -> Style { self.ui.perms.special_user_file }
    fn special_other(&self)      -> Style { self.ui.perms.special_other }
    fn attribute(&self)          -> Style { self.ui.perms.attribute }
}

impl render::SizeColours for Theme {
    fn size(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::*;

        match prefix {
            Some(Kilo | Kibi) => self.ui.size.number_kilo,
            Some(Mega | Mebi) => self.ui.size.number_mega,
            Some(Giga | Gibi) => self.ui.size.number_giga,
            Some(_)           => self.ui.size.number_huge,
            None              => self.ui.size.number_byte,
        }
    }

    fn unit(&self, prefix: Option<unit_prefix::Prefix>) -> Style {
        use unit_prefix::Prefix::*;

        match prefix {
            Some(Kilo | Kibi) => self.ui.size.unit_kilo,
            Some(Mega | Mebi) => self.ui.size.unit_mega,
            Some(Giga | Gibi) => self.ui.size.unit_giga,
            Some(_)           => self.ui.size.unit_huge,
            None              => self.ui.size.unit_byte,
        }
    }

    fn no_size(&self) -> Style { self.ui.punctuation }
    fn major(&self)   -> Style { self.ui.size.major }
    fn comma(&self)   -> Style { self.ui.punctuation }
    fn minor(&self)   -> Style { self.ui.size.minor }
}

#[cfg(unix)]
impl render::UserColours for Theme {
    fn you(&self)           -> Style { self.ui.users.user_you }
    fn someone_else(&self)  -> Style { self.ui.users.user_someone_else }
}

impl FileNameColours for Theme {
    fn normal_arrow(&self)        -> Style { self.ui.punctuation }
    fn broken_symlink(&self)      -> Style { self.ui.broken_symlink }
    fn broken_filename(&self)     -> Style { apply_overlay(self.ui.broken_symlink, self.ui.broken_path_overlay) }
    fn broken_control_char(&self) -> Style { apply_overlay(self.ui.control_char,   self.ui.broken_path_overlay) }
    fn control_char(&self)        -> Style { self.ui.control_char }
    fn symlink_path(&self)        -> Style { self.ui.symlink_path }
    fn executable_file(&self)     -> Style { self.ui.filekinds.executable }

    fn colour_file(&self, file: &File<'_>) -> Style {
        self.exts.colour_file(file).unwrap_or(self.ui.filekinds.normal)
    }
}


/// Some of the styles are **overlays**: although they have the same attribute
/// set as regular styles (foreground and background colours, bold, underline,
/// etc), they’re intended to be used to *amend* existing styles.
///
/// For example, the target path of a broken symlink is displayed in a red,
/// underlined style by default. Paths can contain control characters, so
/// these control characters need to be underlined too, otherwise it looks
/// weird. So instead of having four separate configurable styles for “link
/// path”, “broken link path”, “control character” and “broken control
/// character”, there are styles for “link path”, “control character”, and
/// “broken link overlay”, the latter of which is just set to override the
/// underline attribute on the other two.
fn apply_overlay(mut base: Style, overlay: Style) -> Style {
    if let Some(fg) = overlay.foreground { base.foreground = Some(fg); }
    if let Some(bg) = overlay.background { base.background = Some(bg); }

    if overlay.is_bold          { base.is_bold          = true; }
    if overlay.is_dimmed        { base.is_dimmed        = true; }
    if overlay.is_italic        { base.is_italic        = true; }
    if overlay.is_underline     { base.is_underline     = true; }
    if overlay.is_blink         { base.is_blink         = true; }
    if overlay.is_reverse       { base.is_reverse       = true; }
    if overlay.is_hidden        { base.is_hidden        = true; }
    if overlay.is_strikethrough { base.is_strikethrough = true; }

    base
}
// TODO: move this function to the nu_ansi_term crate


#[cfg(test)]
mod customs_test {
    use super::*;
    use crate::theme::ui_styles::UiStyles;
    use nu_ansi_term::Color::*;

    macro_rules! test {
        ($name:ident:  ls $ls:expr, lx $lx:expr  =>  colours $expected:ident -> $process_expected:expr) => {
            #[test]
            fn $name() {
                let mut $expected = UiStyles::default();
                $process_expected();

                let definitions = Definitions {
                    ls:  Some($ls.into()),
                    lx: Some($lx.into()),
                };

                let mut result = UiStyles::default();
                let (_exts, _reset) = definitions.parse_color_vars(&mut result);
                assert_eq!($expected, result);
            }
        };
        ($name:ident:  ls $ls:expr, lx $lx:expr  =>  exts $mappings:expr) => {
            #[test]
            fn $name() {
                let mappings: Vec<(glob::Pattern, Style)>
                    = $mappings.iter()
                               .map(|t| (glob::Pattern::new(t.0).unwrap(), t.1))
                               .collect();

                let definitions = Definitions {
                    ls:  Some($ls.into()),
                    lx: Some($lx.into()),
                };

                let (result, _reset) = definitions.parse_color_vars(&mut UiStyles::default());
                assert_eq!(ExtensionMappings { mappings }, result);
            }
        };
        ($name:ident:  ls $ls:expr, lx $lx:expr  =>  colours $expected:ident -> $process_expected:expr, exts $mappings:expr) => {
            #[test]
            fn $name() {
                let mut $expected = UiStyles::colourful(false);
                $process_expected();

                let mappings: Vec<(glob::Pattern, Style)>
                    = $mappings.into_iter()
                               .map(|t| (glob::Pattern::new(t.0).unwrap(), t.1))
                               .collect();

                let definitions = Definitions {
                    ls:  Some($ls.into()),
                    lx: Some($lx.into()),
                };

                let mut meh = UiStyles::colourful(false);
                let (result, _reset) = definitions.parse_color_vars(&vars, &mut meh);
                assert_eq!(ExtensionMappings { mappings }, result);
                assert_eq!($expected, meh);
            }
        };
    }


    // LS_COLORS can affect all of these colours:
    test!(ls_di:   ls "di=31", lx ""  =>  colours c -> { c.filekinds.directory    = Red.normal();    });
    test!(ls_ex:   ls "ex=32", lx ""  =>  colours c -> { c.filekinds.executable   = Green.normal();  });
    test!(ls_fi:   ls "fi=33", lx ""  =>  colours c -> { c.filekinds.normal       = Yellow.normal(); });
    test!(ls_pi:   ls "pi=34", lx ""  =>  colours c -> { c.filekinds.pipe         = Blue.normal();   });
    test!(ls_so:   ls "so=35", lx ""  =>  colours c -> { c.filekinds.socket       = Purple.normal(); });
    test!(ls_bd:   ls "bd=36", lx ""  =>  colours c -> { c.filekinds.block_device = Cyan.normal();   });
    test!(ls_cd:   ls "cd=35", lx ""  =>  colours c -> { c.filekinds.char_device  = Purple.normal(); });
    test!(ls_ln:   ls "ln=34", lx ""  =>  colours c -> { c.filekinds.symlink      = Blue.normal();   });
    test!(ls_or:   ls "or=33", lx ""  =>  colours c -> { c.broken_symlink         = Yellow.normal(); });

    // LX_COLORS can affect all those colours too:
    test!(lx_di:  ls "", lx "di=32"  =>  colours c -> { c.filekinds.directory    = Green.normal();  });
    test!(lx_ex:  ls "", lx "ex=33"  =>  colours c -> { c.filekinds.executable   = Yellow.normal(); });
    test!(lx_fi:  ls "", lx "fi=34"  =>  colours c -> { c.filekinds.normal       = Blue.normal();   });
    test!(lx_pi:  ls "", lx "pi=35"  =>  colours c -> { c.filekinds.pipe         = Purple.normal(); });
    test!(lx_so:  ls "", lx "so=36"  =>  colours c -> { c.filekinds.socket       = Cyan.normal();   });
    test!(lx_bd:  ls "", lx "bd=35"  =>  colours c -> { c.filekinds.block_device = Purple.normal(); });
    test!(lx_cd:  ls "", lx "cd=34"  =>  colours c -> { c.filekinds.char_device  = Blue.normal();   });
    test!(lx_ln:  ls "", lx "ln=33"  =>  colours c -> { c.filekinds.symlink      = Yellow.normal(); });
    test!(lx_or:  ls "", lx "or=32"  =>  colours c -> { c.broken_symlink         = Green.normal();  });

    // LX_COLORS will even override options from LS_COLORS:
    test!(ls_lx_di: ls "di=31", lx "di=32"  =>  colours c -> { c.filekinds.directory  = Green.normal();  });
    test!(ls_lx_ex: ls "ex=32", lx "ex=33"  =>  colours c -> { c.filekinds.executable = Yellow.normal(); });
    test!(ls_lx_fi: ls "fi=33", lx "fi=34"  =>  colours c -> { c.filekinds.normal     = Blue.normal();   });

    // But more importantly, LX_COLORS has its own, special list of colours:
    test!(lx_ur:  ls "", lx "ur=38;5;100"  =>  colours c -> { c.perms.user_read           = Fixed(100).normal(); });
    test!(lx_uw:  ls "", lx "uw=38;5;101"  =>  colours c -> { c.perms.user_write          = Fixed(101).normal(); });
    test!(lx_ux:  ls "", lx "ux=38;5;102"  =>  colours c -> { c.perms.user_execute_file   = Fixed(102).normal(); });
    test!(lx_ue:  ls "", lx "ue=38;5;103"  =>  colours c -> { c.perms.user_execute_other  = Fixed(103).normal(); });
    test!(lx_gr:  ls "", lx "gr=38;5;104"  =>  colours c -> { c.perms.group_read          = Fixed(104).normal(); });
    test!(lx_gw:  ls "", lx "gw=38;5;105"  =>  colours c -> { c.perms.group_write         = Fixed(105).normal(); });
    test!(lx_gx:  ls "", lx "gx=38;5;106"  =>  colours c -> { c.perms.group_execute       = Fixed(106).normal(); });
    test!(lx_tr:  ls "", lx "tr=38;5;107"  =>  colours c -> { c.perms.other_read          = Fixed(107).normal(); });
    test!(lx_tw:  ls "", lx "tw=38;5;108"  =>  colours c -> { c.perms.other_write         = Fixed(108).normal(); });
    test!(lx_tx:  ls "", lx "tx=38;5;109"  =>  colours c -> { c.perms.other_execute       = Fixed(109).normal(); });
    test!(lx_su:  ls "", lx "su=38;5;110"  =>  colours c -> { c.perms.special_user_file   = Fixed(110).normal(); });
    test!(lx_sf:  ls "", lx "sf=38;5;111"  =>  colours c -> { c.perms.special_other       = Fixed(111).normal(); });
    test!(lx_xa:  ls "", lx "xa=38;5;112"  =>  colours c -> { c.perms.attribute           = Fixed(112).normal(); });

    test!(lx_sn:  ls "", lx "sn=38;5;113" => colours c -> {
        c.size.number_byte = Fixed(113).normal();
        c.size.number_kilo = Fixed(113).normal();
        c.size.number_mega = Fixed(113).normal();
        c.size.number_giga = Fixed(113).normal();
        c.size.number_huge = Fixed(113).normal();
    });
    test!(lx_sb:  ls "", lx "sb=38;5;114" => colours c -> {
        c.size.unit_byte = Fixed(114).normal();
        c.size.unit_kilo = Fixed(114).normal();
        c.size.unit_mega = Fixed(114).normal();
        c.size.unit_giga = Fixed(114).normal();
        c.size.unit_huge = Fixed(114).normal();
    });

    test!(lx_nb:  ls "", lx "nb=38;5;115"  =>  colours c -> { c.size.number_byte          = Fixed(115).normal(); });
    test!(lx_nk:  ls "", lx "nk=38;5;116"  =>  colours c -> { c.size.number_kilo          = Fixed(116).normal(); });
    test!(lx_nm:  ls "", lx "nm=38;5;117"  =>  colours c -> { c.size.number_mega          = Fixed(117).normal(); });
    test!(lx_ng:  ls "", lx "ng=38;5;118"  =>  colours c -> { c.size.number_giga          = Fixed(118).normal(); });
    test!(lx_nh:  ls "", lx "nh=38;5;119"  =>  colours c -> { c.size.number_huge          = Fixed(119).normal(); });

    test!(lx_ub:  ls "", lx "ub=38;5;115"  =>  colours c -> { c.size.unit_byte            = Fixed(115).normal(); });
    test!(lx_uk:  ls "", lx "uk=38;5;116"  =>  colours c -> { c.size.unit_kilo            = Fixed(116).normal(); });
    test!(lx_um:  ls "", lx "um=38;5;117"  =>  colours c -> { c.size.unit_mega            = Fixed(117).normal(); });
    test!(lx_ug:  ls "", lx "ug=38;5;118"  =>  colours c -> { c.size.unit_giga            = Fixed(118).normal(); });
    test!(lx_uh:  ls "", lx "uh=38;5;119"  =>  colours c -> { c.size.unit_huge            = Fixed(119).normal(); });

    test!(lx_df:  ls "", lx "df=38;5;115"  =>  colours c -> { c.size.major                = Fixed(115).normal(); });
    test!(lx_ds:  ls "", lx "ds=38;5;116"  =>  colours c -> { c.size.minor                = Fixed(116).normal(); });

    test!(lx_uu:  ls "", lx "uu=38;5;117"  =>  colours c -> { c.users.user_you            = Fixed(117).normal(); });
    test!(lx_un:  ls "", lx "un=38;5;118"  =>  colours c -> { c.users.user_someone_else   = Fixed(118).normal(); });
    test!(lx_gu:  ls "", lx "gu=38;5;119"  =>  colours c -> { c.users.group_yours         = Fixed(119).normal(); });
    test!(lx_gn:  ls "", lx "gn=38;5;120"  =>  colours c -> { c.users.group_not_yours     = Fixed(120).normal(); });

    test!(lx_lc:  ls "", lx "lc=38;5;121"  =>  colours c -> { c.links.normal              = Fixed(121).normal(); });
    test!(lx_lm:  ls "", lx "lm=38;5;122"  =>  colours c -> { c.links.multi_link_file     = Fixed(122).normal(); });

    test!(lx_ga:  ls "", lx "ga=38;5;123"  =>  colours c -> { c.vcs.new                   = Fixed(123).normal(); });
    test!(lx_gm:  ls "", lx "gm=38;5;124"  =>  colours c -> { c.vcs.modified              = Fixed(124).normal(); });
    test!(lx_gd:  ls "", lx "gd=38;5;125"  =>  colours c -> { c.vcs.deleted               = Fixed(125).normal(); });
    test!(lx_gv:  ls "", lx "gv=38;5;126"  =>  colours c -> { c.vcs.renamed               = Fixed(126).normal(); });
    test!(lx_gt:  ls "", lx "gt=38;5;127"  =>  colours c -> { c.vcs.typechange            = Fixed(127).normal(); });

    test!(lx_xx:  ls "", lx "xx=38;5;128"  =>  colours c -> { c.punctuation               = Fixed(128).normal(); });
    test!(lx_da:  ls "", lx "da=38;5;129"  =>  colours c -> { c.date                      = Fixed(129).normal(); });
    test!(lx_in:  ls "", lx "in=38;5;130"  =>  colours c -> { c.inode                     = Fixed(130).normal(); });
    test!(lx_bl:  ls "", lx "bl=38;5;131"  =>  colours c -> { c.blocks                    = Fixed(131).normal(); });
    test!(lx_hd:  ls "", lx "hd=38;5;132"  =>  colours c -> { c.header                    = Fixed(132).normal(); });
    test!(lx_lp:  ls "", lx "lp=38;5;133"  =>  colours c -> { c.symlink_path              = Fixed(133).normal(); });
    test!(lx_cc:  ls "", lx "cc=38;5;134"  =>  colours c -> { c.control_char              = Fixed(134).normal(); });
    test!(lx_bo:  ls "", lx "bO=4"         =>  colours c -> { c.broken_path_overlay       = Style::default().underline(); });

    // All the while, LS_COLORS treats them as filenames:
    test!(ls_uu:   ls "uu=38;5;117", lx ""  =>  exts [ ("uu", Fixed(117).normal()) ]);
    test!(ls_un:   ls "un=38;5;118", lx ""  =>  exts [ ("un", Fixed(118).normal()) ]);
    test!(ls_gu:   ls "gu=38;5;119", lx ""  =>  exts [ ("gu", Fixed(119).normal()) ]);
    test!(ls_gn:   ls "gn=38;5;120", lx ""  =>  exts [ ("gn", Fixed(120).normal()) ]);

    // Just like all other keys:
    test!(ls_txt:  ls "*.txt=31",          lx ""  =>  exts [ ("*.txt",      Red.normal())             ]);
    test!(ls_mp3:  ls "*.mp3=38;5;135",    lx ""  =>  exts [ ("*.mp3",      Fixed(135).normal())      ]);
    test!(ls_mak:  ls "Makefile=1;32;4",   lx ""  =>  exts [ ("Makefile",   Green.bold().underline()) ]);
    test!(lx_txt: ls "", lx "*.zip=31"           =>  exts [ ("*.zip",      Red.normal())             ]);
    test!(lx_mp3: ls "", lx "lev.*=38;5;153"     =>  exts [ ("lev.*",      Fixed(153).normal())      ]);
    test!(lx_mak: ls "", lx "Cargo.toml=4;32;1"  =>  exts [ ("Cargo.toml", Green.bold().underline()) ]);

    // Testing whether a glob from LX_COLORS overrides a glob from LS_COLORS
    // can’t be tested here, because they’ll both be added to the same vec

    // Values get separated by colons:
    test!(ls_multi:   ls "*.txt=31:*.rtf=32", lx ""  =>  exts [ ("*.txt", Red.normal()),   ("*.rtf", Green.normal()) ]);
    test!(lx_multi:  ls "", lx "*.tmp=37:*.log=37"  =>  exts [ ("*.tmp", White.normal()), ("*.log", White.normal()) ]);

    test!(ls_five: ls "1*1=31:2*2=32:3*3=1;33:4*4=34;1:5*5=35;4", lx ""  =>  exts [
        ("1*1", Red.normal()), ("2*2", Green.normal()), ("3*3", Yellow.bold()), ("4*4", Blue.bold()), ("5*5", Purple.underline())
    ]);

    // Finally, colours get applied right-to-left:
    test!(ls_overwrite:  ls "pi=31:pi=32:pi=33", lx ""  =>  colours c -> { c.filekinds.pipe = Yellow.normal(); });
    test!(lx_overwrite: ls "", lx "da=36:da=35:da=34"  =>  colours c -> { c.date = Blue.normal(); });
}
