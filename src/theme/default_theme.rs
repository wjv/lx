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
                user_execute_file:   Green.bold().underline(),
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
                group_not_yours:    Style::default(),
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

            punctuation:  Fixed(244).normal(),
            date:         Blue.normal(),
            inode:        Purple.normal(),
            blocks:       Cyan.normal(),
            octal:        Purple.normal(),
            header:       Style::default().underline(),

            symlink_path:         Cyan.normal(),
            control_char:         Red.normal(),
            broken_symlink:       Red.normal(),
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
