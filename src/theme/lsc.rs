use std::iter::Peekable;
use std::ops::FnMut;

use nu_ansi_term::Color::*;
use nu_ansi_term::{Color, Style};

// Parsing the LS_COLORS environment variable into a map of names to Style values.
//
// Note from the original exa codebase: lx (like exa before it) highlights
// output using a theme of Style values, but LS_COLORS contains raw ANSI
// escape codes.  This means we have to parse the codes *back* into Style
// values — which is lossy and fragile.  If a new terminal feature is added,
// lx won’t support it without explicit parsing support, whereas ls would
// handle it transparently.  This is an inherent limitation of the approach.

pub struct LSColors<'var>(pub &'var str);

impl<'var> LSColors<'var> {
    pub fn each_pair<C>(&mut self, mut callback: C)
    where
        C: FnMut(Pair<'var>),
    {
        for next in self.0.split(':') {
            let bits = next.split('=').take(3).collect::<Vec<_>>();

            if bits.len() == 2 && !bits[0].is_empty() && !bits[1].is_empty() {
                callback(Pair {
                    key: bits[0],
                    value: bits[1],
                });
            }
        }
    }
}

fn parse_into_high_colour<'a, I>(iter: &mut Peekable<I>) -> Option<Color>
where
    I: Iterator<Item = &'a str>,
{
    match iter.peek() {
        Some(&"5") => {
            let _ = iter.next();
            if let Some(byte) = iter.next()
                && let Ok(num) = byte.parse()
            {
                return Some(Fixed(num));
            }
        }

        Some(&"2") => {
            let _ = iter.next();
            if let Some(hexes) = iter.next() {
                // Some terminals support R:G:B instead of R;G;B
                // but this clashes with splitting on ‘:’ in each_pair above.
                /*if hexes.contains(':') {
                    let rgb = hexes.splitn(3, ':').collect::<Vec<_>>();
                    if rgb.len() != 3 {
                        return None;
                    }
                    else if let (Ok(r), Ok(g), Ok(b)) = (rgb[0].parse(), rgb[1].parse(), rgb[2].parse()) {
                        return Some(Rgb(r, g, b));
                    }
                }*/

                if let (Some(r), Some(g), Some(b)) = (
                    hexes.parse().ok(),
                    iter.next().and_then(|s| s.parse().ok()),
                    iter.next().and_then(|s| s.parse().ok()),
                ) {
                    return Some(Rgb(r, g, b));
                }
            }
        }

        _ => {}
    }

    None
}

pub struct Pair<'var> {
    pub key: &'var str,
    pub value: &'var str,
}

impl Pair<'_> {
    pub fn to_style(&self) -> Style {
        let mut style = Style::default();
        let mut iter = self.value.split(';').peekable();

        while let Some(num) = iter.next() {
            match num.trim_start_matches('0') {
                // Bold and italic
                "1" => style = style.bold(),
                "2" => style = style.dimmed(),
                "3" => style = style.italic(),
                "4" => style = style.underline(),
                "5" => style = style.blink(),
                // 6 is supposedly a faster blink
                "7" => style = style.reverse(),
                "8" => style = style.hidden(),
                "9" => style = style.strikethrough(),

                // Foreground colours
                "30" => style = style.fg(Black),
                "31" => style = style.fg(Red),
                "32" => style = style.fg(Green),
                "33" => style = style.fg(Yellow),
                "34" => style = style.fg(Blue),
                "35" => style = style.fg(Purple),
                "36" => style = style.fg(Cyan),
                "37" => style = style.fg(White),
                "38" => {
                    if let Some(c) = parse_into_high_colour(&mut iter) {
                        style = style.fg(c);
                    }
                }

                // Background colours
                "40" => style = style.on(Black),
                "41" => style = style.on(Red),
                "42" => style = style.on(Green),
                "43" => style = style.on(Yellow),
                "44" => style = style.on(Blue),
                "45" => style = style.on(Purple),
                "46" => style = style.on(Cyan),
                "47" => style = style.on(White),
                "48" => {
                    if let Some(c) = parse_into_high_colour(&mut iter) {
                        style = style.on(c);
                    }
                }

                _ => { /* ignore the error and do nothing */ }
            }
        }

        style
    }
}

// ── Human-readable colour parser ────────────────────────────────
//
// Parses colour values in the extended format used by [theme] config
// sections.  Accepts space-separated tokens:
//
//   "bold blue"           → bold + blue foreground
//   "cornflowerblue"      → X11 name → RGB(100,149,237)
//   "bold tomato"         → bold + X11 RGB(255,99,71)
//   "#ff8700"             → hex → RGB(255,135,0)
//   "38;5;208"            → falls back to ANSI parser
//   "bold underline"      → modifiers only, default foreground

/// Parse a human-readable colour string into a `Style`.
///
/// Accepts named colours, X11 names, hex `#RRGGBB`/`#RGB`, modifiers
/// (`bold`, `dimmed`, `italic`, `underline`, `strikethrough`), and
/// raw ANSI codes (falls back to `Pair::to_style()`).
pub fn parse_style(value: &str) -> Style {
    let value = value.trim();
    if value.is_empty() {
        return Style::default();
    }

    // If it looks like raw ANSI codes (starts with a digit or contains
    // semicolons without spaces), fall back to the existing parser.
    if looks_like_ansi(value) {
        return Pair { key: "", value }.to_style();
    }

    let mut style = Style::default();
    let mut has_fg = false;

    for token in value.split_whitespace() {
        let lower = token.to_ascii_lowercase();

        // Check modifiers first.
        match lower.as_str() {
            "bold" => {
                style = style.bold();
                continue;
            }
            "dimmed" | "dim" => {
                style = style.dimmed();
                continue;
            }
            "italic" => {
                style = style.italic();
                continue;
            }
            "underline" => {
                style = style.underline();
                continue;
            }
            "strikethrough" => {
                style = style.strikethrough();
                continue;
            }
            "blink" => {
                style = style.blink();
                continue;
            }
            "reverse" => {
                style = style.reverse();
                continue;
            }
            "hidden" => {
                style = style.hidden();
                continue;
            }
            _ => {}
        }

        // Check basic ANSI colour names.
        if let Some(colour) = basic_colour(&lower) {
            style = style.fg(colour);
            has_fg = true;
            continue;
        }

        // Check hex notation.
        if let Some(colour) = parse_hex(token) {
            style = style.fg(colour);
            has_fg = true;
            continue;
        }

        // Check X11/CSS colour names.
        if let Some(&(r, g, b)) = X11_COLOURS.get(lower.as_str()) {
            style = style.fg(Rgb(r, g, b));
            has_fg = true;
            continue;
        }

        // If it looks like ANSI codes embedded in the string (e.g.
        // "bold 38;5;208"), parse just this token as ANSI.
        if token.contains(';') || token.chars().all(|c| c.is_ascii_digit()) {
            let sub = Pair {
                key: "",
                value: token,
            }
            .to_style();
            if sub.foreground.is_some() && !has_fg {
                style.foreground = sub.foreground;
                has_fg = true;
            }
            if sub.background.is_some() {
                style.background = sub.background;
            }
            // Merge attributes.
            if sub.is_bold {
                style = style.bold();
            }
            if sub.is_dimmed {
                style = style.dimmed();
            }
            if sub.is_italic {
                style = style.italic();
            }
            if sub.is_underline {
                style = style.underline();
            }
            if sub.is_blink {
                style = style.blink();
            }
            if sub.is_reverse {
                style = style.reverse();
            }
            if sub.is_hidden {
                style = style.hidden();
            }
            if sub.is_strikethrough {
                style = style.strikethrough();
            }
            continue;
        }

        // Unknown token — silently ignore.
        log::debug!("Unknown colour token: {token:?}");
    }

    style
}

/// Does this string look like raw ANSI codes rather than named colours?
fn looks_like_ansi(s: &str) -> bool {
    // Contains semicolons and no spaces → definitely ANSI (e.g. "38;5;208")
    s.contains(';') && !s.contains(' ')
}

/// Map basic ANSI colour names to `Color` values.
fn basic_colour(name: &str) -> Option<Color> {
    match name {
        "black" => Some(Black),
        "red" => Some(Red),
        "green" => Some(Green),
        "yellow" => Some(Yellow),
        "blue" => Some(Blue),
        "purple" | "magenta" => Some(Purple),
        "cyan" => Some(Cyan),
        "white" => Some(White),
        _ => None,
    }
}

/// Parse a hex colour (`#RRGGBB` or `#RGB`) into an `Rgb` colour.
fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Rgb(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Rgb(r, g, b))
        }
        _ => None,
    }
}

/// X11/CSS colour names → RGB values.
///
/// The full set of standard X11 colour names (~148 entries).  These
/// are case-insensitive; lookup is done on the lowercased name.
static X11_COLOURS: phf::Map<&'static str, (u8, u8, u8)> = phf::phf_map! {
    "aliceblue" => (240, 248, 255),
    "antiquewhite" => (250, 235, 215),
    "aqua" => (0, 255, 255),
    "aquamarine" => (127, 255, 212),
    "azure" => (240, 255, 255),
    "beige" => (245, 245, 220),
    "bisque" => (255, 228, 196),
    "blanchedalmond" => (255, 235, 205),
    "blueviolet" => (138, 43, 226),
    "brown" => (165, 42, 42),
    "burlywood" => (222, 184, 135),
    "cadetblue" => (95, 158, 160),
    "chartreuse" => (127, 255, 0),
    "chocolate" => (210, 105, 30),
    "coral" => (255, 127, 80),
    "cornflowerblue" => (100, 149, 237),
    "cornsilk" => (255, 248, 220),
    "crimson" => (220, 20, 60),
    "darkblue" => (0, 0, 139),
    "darkcyan" => (0, 139, 139),
    "darkgoldenrod" => (184, 134, 11),
    "darkgray" => (169, 169, 169),
    "darkgrey" => (169, 169, 169),
    "darkgreen" => (0, 100, 0),
    "darkkhaki" => (189, 183, 107),
    "darkmagenta" => (139, 0, 139),
    "darkolivegreen" => (85, 107, 47),
    "darkorange" => (255, 140, 0),
    "darkorchid" => (153, 50, 204),
    "darkred" => (139, 0, 0),
    "darksalmon" => (233, 150, 122),
    "darkseagreen" => (143, 188, 143),
    "darkslateblue" => (72, 61, 139),
    "darkslategray" => (47, 79, 79),
    "darkslategrey" => (47, 79, 79),
    "darkturquoise" => (0, 206, 209),
    "darkviolet" => (148, 0, 211),
    "deeppink" => (255, 20, 147),
    "deepskyblue" => (0, 191, 255),
    "dimgray" => (105, 105, 105),
    "dimgrey" => (105, 105, 105),
    "dodgerblue" => (30, 144, 255),
    "firebrick" => (178, 34, 34),
    "floralwhite" => (255, 250, 240),
    "forestgreen" => (34, 139, 34),
    "fuchsia" => (255, 0, 255),
    "gainsboro" => (220, 220, 220),
    "ghostwhite" => (248, 248, 255),
    "gold" => (255, 215, 0),
    "goldenrod" => (218, 165, 32),
    "gray" => (128, 128, 128),
    "grey" => (128, 128, 128),
    "greenyellow" => (173, 255, 47),
    "honeydew" => (240, 255, 240),
    "hotpink" => (255, 105, 180),
    "indianred" => (205, 92, 92),
    "indigo" => (75, 0, 130),
    "ivory" => (255, 255, 240),
    "khaki" => (240, 230, 140),
    "lavender" => (230, 230, 250),
    "lavenderblush" => (255, 240, 245),
    "lawngreen" => (124, 252, 0),
    "lemonchiffon" => (255, 250, 205),
    "lightblue" => (173, 216, 230),
    "lightcoral" => (240, 128, 128),
    "lightcyan" => (224, 255, 255),
    "lightgoldenrodyellow" => (250, 250, 210),
    "lightgray" => (211, 211, 211),
    "lightgrey" => (211, 211, 211),
    "lightgreen" => (144, 238, 144),
    "lightpink" => (255, 182, 193),
    "lightsalmon" => (255, 160, 122),
    "lightseagreen" => (32, 178, 170),
    "lightskyblue" => (135, 206, 250),
    "lightslategray" => (119, 136, 153),
    "lightslategrey" => (119, 136, 153),
    "lightsteelblue" => (176, 196, 222),
    "lightyellow" => (255, 255, 224),
    "lime" => (0, 255, 0),
    "limegreen" => (50, 205, 50),
    "linen" => (250, 240, 230),
    "maroon" => (128, 0, 0),
    "mediumaquamarine" => (102, 205, 170),
    "mediumblue" => (0, 0, 205),
    "mediumorchid" => (186, 85, 211),
    "mediumpurple" => (147, 111, 219),
    "mediumseagreen" => (60, 179, 113),
    "mediumslateblue" => (123, 104, 238),
    "mediumspringgreen" => (0, 250, 154),
    "mediumturquoise" => (72, 209, 204),
    "mediumvioletred" => (199, 21, 133),
    "midnightblue" => (25, 25, 112),
    "mintcream" => (245, 255, 250),
    "mistyrose" => (255, 228, 225),
    "moccasin" => (255, 228, 181),
    "navajowhite" => (255, 222, 173),
    "navy" => (0, 0, 128),
    "oldlace" => (253, 245, 230),
    "olive" => (128, 128, 0),
    "olivedrab" => (107, 142, 35),
    "orange" => (255, 165, 0),
    "orangered" => (255, 69, 0),
    "orchid" => (218, 112, 214),
    "palegoldenrod" => (238, 232, 170),
    "palegreen" => (152, 251, 152),
    "paleturquoise" => (175, 238, 238),
    "palevioletred" => (219, 112, 147),
    "papayawhip" => (255, 239, 213),
    "peachpuff" => (255, 218, 185),
    "peru" => (205, 133, 63),
    "pink" => (255, 192, 203),
    "plum" => (221, 160, 221),
    "powderblue" => (176, 224, 230),
    "rebeccapurple" => (102, 51, 153),
    "rosybrown" => (188, 143, 143),
    "royalblue" => (65, 105, 225),
    "saddlebrown" => (139, 69, 19),
    "salmon" => (250, 128, 114),
    "sandybrown" => (244, 164, 96),
    "seagreen" => (46, 139, 87),
    "seashell" => (255, 245, 238),
    "sienna" => (160, 82, 45),
    "silver" => (192, 192, 192),
    "skyblue" => (135, 206, 235),
    "slateblue" => (106, 90, 205),
    "slategray" => (112, 128, 144),
    "slategrey" => (112, 128, 144),
    "snow" => (255, 250, 250),
    "springgreen" => (0, 255, 127),
    "steelblue" => (70, 130, 180),
    "tan" => (210, 180, 140),
    "teal" => (0, 128, 128),
    "thistle" => (216, 191, 216),
    "tomato" => (255, 99, 71),
    "turquoise" => (64, 224, 208),
    "violet" => (238, 130, 238),
    "wheat" => (245, 222, 179),
    "whitesmoke" => (245, 245, 245),
    "yellowgreen" => (154, 205, 50),
};

#[cfg(test)]
mod ansi_test {
    use super::*;
    use nu_ansi_term::Style;

    macro_rules! test {
        ($name:ident: $input:expr => $result:expr) => {
            #[test]
            fn $name() {
                assert_eq!(
                    Pair {
                        key: "",
                        value: $input
                    }
                    .to_style(),
                    $result
                );
            }
        };
    }

    // Styles
    test!(bold:  "1"         => Style::default().bold());
    test!(bold2: "01"        => Style::default().bold());
    test!(under: "4"         => Style::default().underline());
    test!(unde2: "04"        => Style::default().underline());
    test!(both:  "1;4"       => Style::default().bold().underline());
    test!(both2: "01;04"     => Style::default().bold().underline());
    test!(fg:    "31"        => Red.normal());
    test!(bg:    "43"        => Style::default().on(Yellow));
    test!(bfg:   "31;43"     => Red.on(Yellow));
    test!(bfg2:  "0031;0043" => Red.on(Yellow));
    test!(all:   "43;31;1;4" => Red.on(Yellow).bold().underline());
    test!(again: "1;1;1;1;1" => Style::default().bold());

    // Failure cases
    test!(empty: ""          => Style::default());
    test!(semis: ";;;;;;"    => Style::default());
    test!(nines: "99999999"  => Style::default());
    test!(word:  "GREEN"     => Style::default());

    // Higher colours
    test!(hifg:  "38;5;149"  => Fixed(149).normal());
    test!(hibg:  "48;5;1"    => Style::default().on(Fixed(1)));
    test!(hibo:  "48;5;1;1"  => Style::default().on(Fixed(1)).bold());
    test!(hiund: "4;48;5;1"  => Style::default().on(Fixed(1)).underline());

    test!(rgb:   "38;2;255;100;0"     => Style::default().fg(Rgb(255, 100, 0)));
    test!(rgbi:  "38;2;255;100;0;3"   => Style::default().fg(Rgb(255, 100, 0)).italic());
    test!(rgbbg: "48;2;255;100;0"     => Style::default().on(Rgb(255, 100, 0)));
    test!(rgbbi: "48;2;255;100;0;3"   => Style::default().on(Rgb(255, 100, 0)).italic());

    test!(fgbg:  "38;5;121;48;5;212"  => Fixed(121).on(Fixed(212)));
    test!(bgfg:  "48;5;121;38;5;212"  => Fixed(212).on(Fixed(121)));
    test!(toohi: "48;5;999"           => Style::default());
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! test {
        ($name:ident: $input:expr => $result:expr) => {
            #[test]
            fn $name() {
                let mut lscs = Vec::new();
                LSColors($input).each_pair(|p| lscs.push((p.key.clone(), p.to_style())));
                assert_eq!(lscs, $result.to_vec());
            }
        };
    }

    // Bad parses
    test!(empty:    ""       => []);
    test!(jibber:   "blah"   => []);

    test!(equals:     "="    => []);
    test!(starts:     "=di"  => []);
    test!(ends:     "id="    => []);

    // Foreground colours
    test!(green:   "cb=32"   => [ ("cb", Green.normal()) ]);
    test!(red:     "di=31"   => [ ("di", Red.normal()) ]);
    test!(blue:    "la=34"   => [ ("la", Blue.normal()) ]);

    // Background colours
    test!(yellow:  "do=43"   => [ ("do", Style::default().on(Yellow)) ]);
    test!(purple:  "re=45"   => [ ("re", Style::default().on(Purple)) ]);
    test!(cyan:    "mi=46"   => [ ("mi", Style::default().on(Cyan)) ]);

    // Bold and underline
    test!(bold:    "fa=1"    => [ ("fa", Style::default().bold()) ]);
    test!(under:   "so=4"    => [ ("so", Style::default().underline()) ]);
    test!(both:    "la=1;4"  => [ ("la", Style::default().bold().underline()) ]);

    // More and many
    test!(more:  "me=43;21;55;34:yu=1;4;1"  => [ ("me", Blue.on(Yellow)), ("yu", Style::default().bold().underline()) ]);
    test!(many:  "red=31:green=32:blue=34"  => [ ("red", Red.normal()), ("green", Green.normal()), ("blue", Blue.normal()) ]);
}

#[cfg(test)]
mod parse_style_test {
    use super::*;
    use nu_ansi_term::Style;

    #[test]
    fn empty_string() {
        assert_eq!(parse_style(""), Style::default());
    }

    #[test]
    fn named_colour() {
        assert_eq!(parse_style("blue"), Blue.normal());
    }

    #[test]
    fn named_colour_case_insensitive() {
        assert_eq!(parse_style("Blue"), Blue.normal());
        assert_eq!(parse_style("BLUE"), Blue.normal());
    }

    #[test]
    fn bold_named() {
        assert_eq!(parse_style("bold blue"), Blue.bold());
    }

    #[test]
    fn named_bold_order() {
        // Modifier after colour should also work.
        assert_eq!(parse_style("blue bold"), Blue.bold());
    }

    #[test]
    fn multiple_modifiers() {
        assert_eq!(
            parse_style("bold underline"),
            Style::default().bold().underline()
        );
    }

    #[test]
    fn bold_underline_colour() {
        assert_eq!(parse_style("bold underline red"), Red.bold().underline());
    }

    #[test]
    fn magenta_alias() {
        assert_eq!(parse_style("magenta"), Purple.normal());
    }

    #[test]
    fn hex_colour_6() {
        assert_eq!(
            parse_style("#ff8700"),
            Style::default().fg(Rgb(255, 135, 0))
        );
    }

    #[test]
    fn hex_colour_3() {
        // #f00 → #ff0000
        assert_eq!(parse_style("#f00"), Style::default().fg(Rgb(255, 0, 0)));
    }

    #[test]
    fn bold_hex() {
        assert_eq!(
            parse_style("bold #ff8700"),
            Style::default().fg(Rgb(255, 135, 0)).bold()
        );
    }

    #[test]
    fn x11_tomato() {
        assert_eq!(parse_style("tomato"), Style::default().fg(Rgb(255, 99, 71)));
    }

    #[test]
    fn bold_x11() {
        assert_eq!(
            parse_style("bold tomato"),
            Style::default().fg(Rgb(255, 99, 71)).bold()
        );
    }

    #[test]
    fn x11_cornflowerblue() {
        assert_eq!(
            parse_style("cornflowerblue"),
            Style::default().fg(Rgb(100, 149, 237))
        );
    }

    #[test]
    fn ansi_fallback_256() {
        // Pure ANSI code string falls back to Pair::to_style()
        assert_eq!(parse_style("38;5;208"), Fixed(208).normal());
    }

    #[test]
    fn ansi_fallback_with_bold() {
        assert_eq!(parse_style("1;38;5;208"), Fixed(208).bold());
    }

    #[test]
    fn bold_with_inline_ansi() {
        // "bold 38;5;208" — modifier word + ANSI code
        assert_eq!(parse_style("bold 38;5;208"), Fixed(208).bold());
    }

    #[test]
    fn dimmed_alias() {
        assert_eq!(parse_style("dim green"), Green.dimmed());
    }

    #[test]
    fn unknown_token_ignored() {
        // Unknown tokens are silently skipped.
        assert_eq!(parse_style("bold frobnicate blue"), Blue.bold());
    }
}
