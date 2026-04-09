use nu_ansi_term::Style;
use nu_ansi_term::Color::*;

use crate::theme::ColourScale;
use crate::theme::ui_styles::*;


impl UiStyles {
    pub fn default_theme(scale: ColourScale) -> Self {
        Self {
            colourful: true,

            filekinds: FileKinds {
                normal:       Style::default(),
                directory:    Blue.bold(),
                symlink:      Cyan.normal(),
                pipe:         Yellow.normal(),
                block_device: Yellow.bold(),
                char_device:  Yellow.bold(),
                socket:       Red.bold(),
                special:      Yellow.normal(),
                executable:   Green.bold(),
            },

            perms: Permissions {
                user_read:           Yellow.bold(),
                user_write:          Red.bold(),
                user_execute_file:   Green.bold(),
                user_execute_other:  Green.bold(),

                group_read:          Yellow.normal(),
                group_write:         Red.normal(),
                group_execute:       Green.normal(),

                other_read:          Yellow.normal(),
                other_write:         Red.normal(),
                other_execute:       Green.normal(),

                special_user_file:   Purple.normal(),
                special_other:       Purple.normal(),

                attribute:           Style::default(),
            },

            size: Size::colourful(scale),

            users: Users {
                user_you:           Yellow.bold(),
                user_someone_else:  Style::default(),
                group_yours:        Yellow.bold(),
                group_member:       Yellow.normal(),
                group_not_yours:    Style::default(),
                uid_you:            Cyan.bold(),
                uid_someone_else:   Style::default(),
                gid_yours:          Cyan.bold(),
                gid_member:         Cyan.normal(),
                gid_not_yours:      Style::default(),
            },

            links: Links {
                normal:          Red.bold(),
                multi_link_file: Red.on(Yellow),
            },

            vcs: Git {
                new:         Green.normal(),
                modified:    Blue.normal(),
                deleted:     Red.normal(),
                renamed:     Yellow.normal(),
                typechange:  Purple.normal(),
                ignored:     Style::default().dimmed(),
                conflicted:  Red.normal(),
            },

            // ANSI has no "subdued grey" — punctuation collapses to
            // the terminal foreground.  lx-256 and lx-24bit will use
            // Fixed/RGB greys for visual subordination.
            punctuation:  Style::default(),
            // ANSI date "gradient" collapses to a single colour —
            // matches the historical exa behaviour.  The age-based
            // gradient is reserved for lx-256 and lx-24bit.
            date: {
                let mut d = DateAge::default();
                d.set_all(Blue.normal());
                d
            },
            inode:        Purple.normal(),
            blocks:       Cyan.normal(),
            octal:        Purple.normal(),
            flags:        Yellow.normal(),
            header:       Style::default().underline(),

            symlink_path:         Cyan.normal(),
            control_char:         Red.normal(),
            broken_symlink:       Red.normal(),
            broken_path_overlay:  Style::default().underline(),
        }
    }
}


impl UiStyles {
    /// The compiled-in `lx-256` theme: refined, recognisably
    /// exa-derived, but using the 256-colour xterm palette for
    /// smoother gradients and tasteful chrome.  Designed to look
    /// good on both light and dark backgrounds.
    pub fn lx_256_theme() -> Self {
        Self {
            colourful: true,

            filekinds: FileKinds {
                normal:       Style::default(),
                directory:    Fixed(33).bold(),    // soft blue
                symlink:      Fixed(38).normal(),  // turquoise
                pipe:         Fixed(178).normal(), // muted gold
                block_device: Fixed(178).bold(),
                char_device:  Fixed(178).bold(),
                socket:       Fixed(167).bold(),   // salmon
                special:      Fixed(178).normal(),
                executable:   Fixed(41).bold(),    // medium green
            },

            perms: Permissions {
                user_read:           Fixed(178).bold(),
                user_write:          Fixed(167).bold(),
                user_execute_file:   Fixed(41).bold(),
                user_execute_other:  Fixed(41).bold(),

                group_read:          Fixed(178).normal(),
                group_write:         Fixed(167).normal(),
                group_execute:       Fixed(41).normal(),

                other_read:          Fixed(178).normal(),
                other_write:         Fixed(167).normal(),
                other_execute:       Fixed(41).normal(),

                special_user_file:   Fixed(141).normal(), // mauve
                special_other:       Fixed(141).normal(),

                attribute:           Style::default(),
            },

            // Smooth size gradient: green → yellow → orange → red.
            // Mid-tone palette: visible on both light and dark.
            size: Size {
                major:  Fixed(41).bold(),
                minor:  Fixed(41).normal(),

                number_byte: Fixed(76).normal(),   // chartreuse
                number_kilo: Fixed(142).normal(),  // mid olive
                number_mega: Fixed(178).normal(),  // gold
                number_giga: Fixed(172).normal(),  // orange-3
                number_huge: Fixed(160).normal(),  // red-3

                unit_byte: Fixed(244).normal(),
                unit_kilo: Fixed(244).normal(),
                unit_mega: Fixed(244).normal(),
                unit_giga: Fixed(244).normal(),
                unit_huge: Fixed(244).normal(),
            },

            users: Users {
                user_you:           Fixed(178).bold(),  // gold
                user_someone_else:  Style::default(),
                group_yours:        Fixed(178).bold(),
                group_member:       Fixed(178).normal(),
                group_not_yours:    Style::default(),
                uid_you:            Fixed(38).bold(),   // turquoise
                uid_someone_else:   Style::default(),
                gid_yours:          Fixed(38).bold(),
                gid_member:         Fixed(38).normal(),
                gid_not_yours:      Style::default(),
            },

            links: Links {
                normal:          Fixed(167).bold(),
                multi_link_file: Style::default().on(Fixed(178)),
            },

            vcs: Git {
                new:         Fixed(41).normal(),
                modified:    Fixed(33).normal(),
                deleted:     Fixed(167).normal(),
                renamed:     Fixed(178).normal(),
                typechange:  Fixed(141).normal(),
                ignored:     Fixed(244).normal(),
                conflicted:  Fixed(167).bold(),
            },

            punctuation:  Fixed(244).normal(),  // medium grey
            // Smooth date gradient: cyan → blue → grey.
            // Mid-tone blues: visible on both light and dark.
            date: DateAge {
                now:   Fixed(38).bold(),    // turquoise
                today: Fixed(38).normal(),  // turquoise
                week:  Fixed(32).normal(),  // deeper teal
                month: Fixed(27).normal(),  // royal blue
                year:  Fixed(244).normal(), // medium grey
                old:   Fixed(240).normal(), // dark grey
            },
            inode:        Fixed(141).normal(),
            blocks:       Fixed(38).normal(),
            octal:        Fixed(141).normal(),
            flags:        Fixed(178).normal(),
            header:       Fixed(33).bold().underline(),  // soft blue

            symlink_path:         Fixed(38).normal(),
            control_char:         Fixed(167).normal(),
            broken_symlink:       Fixed(167).normal(),
            broken_path_overlay:  Style::default().underline(),
        }
    }
}


impl Size {
    pub fn colourful(scale: ColourScale) -> Self {
        match scale {
            ColourScale::None     => Self::colourful_fixed(),
            ColourScale::Scale16  => Self::colourful_gradient(),
            ColourScale::Scale256 => Self::colourful_gradient_256(),
        }
    }

    fn colourful_fixed() -> Self {
        Self {
            major:  Green.bold(),
            minor:  Green.normal(),

            number_byte: Green.bold(),
            number_kilo: Green.bold(),
            number_mega: Green.bold(),
            number_giga: Green.bold(),
            number_huge: Green.bold(),

            unit_byte: Green.normal(),
            unit_kilo: Green.normal(),
            unit_mega: Green.normal(),
            unit_giga: Green.normal(),
            unit_huge: Green.normal(),
        }
    }

    fn colourful_gradient() -> Self {
        Self {
            major:  Green.bold(),
            minor:  Green.normal(),

            number_byte: Green.normal(),
            number_kilo: Green.bold(),
            number_mega: Yellow.normal(),
            number_giga: Red.normal(),
            number_huge: Purple.normal(),

            unit_byte: Green.normal(),
            unit_kilo: Green.bold(),
            unit_mega: Yellow.normal(),
            unit_giga: Red.normal(),
            unit_huge: Purple.normal(),
        }
    }

    /// 256-colour gradient.
    fn colourful_gradient_256() -> Self {
        Self {
            major:  Green.bold(),
            minor:  Green.normal(),

            number_byte: Fixed(118).normal(),
            number_kilo: Fixed(190).normal(),
            number_mega: Fixed(226).normal(),
            number_giga: Fixed(220).normal(),
            number_huge: Fixed(214).normal(),

            unit_byte: Green.normal(),
            unit_kilo: Green.normal(),
            unit_mega: Green.normal(),
            unit_giga: Green.normal(),
            unit_huge: Green.normal(),
        }
    }
}
