//! Smooth-gradient lookup tables and the build helper that
//! produces them.
//!
//! The smooth-gradient feature (`--smooth`) replaces a column's
//! discrete tier rendering with a 256-stop precomputed LUT,
//! interpolated between the theme's anchor colours in Oklab
//! space.  At render time the caller maps each file's size or
//! age onto one of the 256 buckets with a single integer
//! lookup — no colour-space maths in the hot path.
//!
//! Interpolation formulas come from Björn Ottosson, *"A
//! perceptual color space for image processing"* (2020),
//! <https://bottosson.github.io/posts/oklab/>.  See
//! `src/theme/oklab.rs` for the conversion primitives.
//!
//! LUT construction runs once per `lx` invocation, in
//! `UiStyles::apply_gradient_flags`, for each gradient-capable
//! column whose theme anchors are all 24-bit (`is_smoothable()`
//! returned true) and whose column gradient flag is on.

use nu_ansi_term::{Color, Style};

use super::oklab::lerp_oklab;
use super::ui_styles::{DateAge, Size};

/// Number of stops in a smooth-gradient lookup table.  Far beyond
/// the eye's ability to distinguish adjacent colours on a text
/// column, and matches exactly one `u8` worth of bucket index.
pub const LUT_SIZE: usize = 256;

/// A LUT as stored in `SmoothLuts` and consumed by the renderer.
///
/// Boxed so the 256 × `Style` payload sits on the heap and the
/// owning struct (`UiStyles`) stays small and cheap to move.
pub type SmoothLut = Box<[Style; LUT_SIZE]>;

/// Per-column smooth-gradient LUTs.  A `None` for a column means
/// one of:
///
/// - smooth mode is off entirely (`--smooth` was not passed), or
/// - the column's gradient is off (`--gradient=none`, etc.), or
/// - the column's theme anchors aren't all 24-bit so the smooth
///   predicate rejected it.
///
/// In all three cases the renderer falls through to the discrete
/// per-tier fields on `Size`/`DateAge`.
#[derive(Debug, Default, PartialEq)]
pub struct SmoothLuts {
    pub size: Option<SmoothLut>,
    pub modified: Option<SmoothLut>,
    pub accessed: Option<SmoothLut>,
    pub changed: Option<SmoothLut>,
    pub created: Option<SmoothLut>,
}

/// Build a 256-stop LUT by interpolating between a sequence of
/// `(position, Style)` anchors in Oklab space.
///
/// # Invariants expected from the caller
///
/// - `anchors` is non-empty and sorted by strictly ascending
///   position.
/// - The first anchor's position is `0.0` and the last is `1.0`.
/// - Every anchor's foreground is `Some(Color::Rgb(...))` —
///   callers must gate on `Size::is_smoothable` /
///   `DateAge::is_smoothable` before calling.
///
/// Violations are caught by `debug_assert!` in debug builds;
/// release builds will still return *something*, but the colours
/// are undefined.
///
/// # Attribute handling
///
/// Every interpolated stop inherits bold/underline/italic/etc.
/// from the **earlier** (lower-position) anchor of its bracketing
/// pair — the "bias-toward-hotter" rule.  A stop that lands
/// exactly on an interior anchor's position uses that anchor's
/// own full style.  The result is that, for example, `lx-24bit`'s
/// bold `date_now` extends across the whole `< 1 hour` range and
/// flips to plain at the `today` boundary.
pub fn build_smooth_lut(anchors: &[(f32, Style)]) -> SmoothLut {
    debug_assert!(!anchors.is_empty(), "anchors must be non-empty");
    debug_assert!(
        (anchors[0].0 - 0.0).abs() < f32::EPSILON,
        "first anchor must sit at position 0.0",
    );
    debug_assert!(
        (anchors[anchors.len() - 1].0 - 1.0).abs() < f32::EPSILON,
        "last anchor must sit at position 1.0",
    );

    // Extract the RGB foregrounds once so we don't re-match on
    // `Color::Rgb` inside the per-stop loop.
    let rgbs: Vec<(u8, u8, u8)> = anchors
        .iter()
        .map(|(_, style)| {
            if let Some(Color::Rgb(r, g, b)) = style.foreground {
                (r, g, b)
            } else {
                debug_assert!(
                    false,
                    "build_smooth_lut called with a non-Rgb anchor; \
                     caller must gate on is_smoothable()",
                );
                (0, 0, 0)
            }
        })
        .collect();

    // Allocate and fill the LUT.
    let mut lut: SmoothLut = vec![Style::default(); LUT_SIZE]
        .into_boxed_slice()
        .try_into()
        .expect("LUT_SIZE-length Vec should convert to a fixed array");

    for (i, stop) in lut.iter_mut().enumerate() {
        let position = i as f32 / (LUT_SIZE - 1) as f32;

        // Find the bracketing anchor pair.  Ties go to the
        // earlier anchor: a stop at exactly anchors[j]'s position
        // uses anchors[j]'s own full style (see the attribute
        // rule in the docstring).
        let lo_idx = anchors
            .iter()
            .rposition(|(pos, _)| *pos <= position)
            .unwrap_or(0);
        let hi_idx = (lo_idx + 1).min(anchors.len() - 1);

        let (lo_pos, lo_style) = anchors[lo_idx];
        let (hi_pos, _) = anchors[hi_idx];

        // Past the last anchor — no interpolation.
        if lo_idx == hi_idx {
            *stop = lo_style;
            continue;
        }

        let span = hi_pos - lo_pos;
        let t = if span.abs() < f32::EPSILON {
            0.0
        } else {
            ((position - lo_pos) / span).clamp(0.0, 1.0)
        };

        let (r, g, b) = lerp_oklab(rgbs[lo_idx], rgbs[hi_idx], t);

        // Bias-toward-hotter: take the earlier anchor's full
        // style, then overwrite the foreground with the Oklab-
        // lerped RGB.
        let mut style = lo_style;
        style.foreground = Some(Color::Rgb(r, g, b));
        *stop = style;
    }

    lut
}

/// Map a file size in bytes onto the smooth-gradient scale.
///
/// Returns a float in `[0.0, 1.0]` log-scaled such that each
/// builtin size tier lands exactly on its LUT anchor position:
///
/// | bytes                | position |
/// |----------------------|----------|
/// | 0                    | 0.0 (clamped) |
/// | 1 (`byte`)           | 0.0      |
/// | 1 KiB (`kilo`)       | 0.25     |
/// | 1 MiB (`mega`)       | 0.5      |
/// | 1 GiB (`giga`)       | 0.75     |
/// | 1 TiB (`huge`)       | 1.0      |
/// | > 1 TiB              | 1.0 (clamped) |
///
/// The anchors are powers of 1024, so the mapping is a single
/// `log2` divided by 40 (four decades of `log2(1024)` = 10 each).
/// No piecewise logic needed.
pub fn size_to_position(bytes: u64) -> f32 {
    const HUGE_ANCHOR: u64 = 1_u64 << 40; // 1 TiB
    const LOG2_HUGE: f32 = 40.0;

    if bytes == 0 {
        return 0.0;
    }
    if bytes > HUGE_ANCHOR {
        return 1.0;
    }
    ((bytes as f32).log2() / LOG2_HUGE).clamp(0.0, 1.0)
}

/// Map a file age in seconds onto the smooth-gradient scale.
///
/// Returns a float in `[0.0, 1.0]` log-scaled such that each
/// builtin age tier lands exactly on its LUT anchor position:
///
/// | age (seconds)    | position | anchor              |
/// |------------------|----------|---------------------|
/// | 0                | 0.0 (clamped) |                |
/// | 1                | 0.0      | `now`               |
/// | 3600 (1 hour)    | 0.2      | `today`             |
/// | 86400 (1 day)    | 0.4      | `week`              |
/// | 604800 (1 week)  | 0.6      | `month`             |
/// | 2592000 (30 d)   | 0.8      | `year`              |
/// | 31536000 (1 yr)  | 1.0      | `old`               |
/// | > 1 year         | 1.0 (clamped) |                |
///
/// The anchors aren't evenly log-spaced (1 sec → 1 hour is
/// ~11.8 log₂ units; 1 hour → 1 day is ~4.6), so the mapping is
/// piecewise linear in `log2(age)` between adjacent anchors.
/// Each anchor pair gets a fifth of the position range (0.2).
///
/// Anchor positions match the "upper boundary of each tier" rule:
/// a file exactly 1 hour old lands on the `today` anchor (the
/// upper bound of the `now` tier range), and so on.
pub fn age_to_position(age_secs: u64) -> f32 {
    const ANCHORS: [u64; 6] = [
        1,          // `now`   — scale origin (anything ≤ 1 sec clamps here)
        3_600,      // `today` — 1 hour
        86_400,     // `week`  — 1 day
        604_800,    // `month` — 1 week
        2_592_000,  // `year`  — 30 days
        31_536_000, // `old`  — 1 year
    ];
    const POSITIONS: [f32; 6] = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

    if age_secs <= ANCHORS[0] {
        return 0.0;
    }
    if age_secs >= ANCHORS[5] {
        return 1.0;
    }

    // Find the bracketing anchor pair and interpolate linearly in
    // log2(age) space.  Anchor count is tiny (5 segments), so a
    // linear search is fine.
    for i in 0..5 {
        if age_secs <= ANCHORS[i + 1] {
            let lo_log = (ANCHORS[i] as f32).log2();
            let hi_log = (ANCHORS[i + 1] as f32).log2();
            let t = ((age_secs as f32).log2() - lo_log) / (hi_log - lo_log);
            return POSITIONS[i] + t * (POSITIONS[i + 1] - POSITIONS[i]);
        }
    }
    1.0
}

/// Anchor layout for a `Size` column: five tiers evenly spaced
/// along [0.0, 1.0] by tier index.  The log-scale mapping from
/// bytes to position lives in [`size_to_position`]; the LUT
/// itself is perceptually uniform in colour, not in bytes.
pub(crate) fn size_anchors(size: &Size) -> [(f32, Style); 5] {
    [
        (0.00, size.number_byte),
        (0.25, size.number_kilo),
        (0.50, size.number_mega),
        (0.75, size.number_giga),
        (1.00, size.number_huge),
    ]
}

/// Anchor layout for a `DateAge` column: six tiers evenly
/// spaced along [0.0, 1.0] by tier index.  The log-scale mapping
/// from age to position lives in the renderer.
pub(crate) fn date_anchors(date: &DateAge) -> [(f32, Style); 6] {
    [
        (0.0, date.now),
        (0.2, date.today),
        (0.4, date.week),
        (0.6, date.month),
        (0.8, date.year),
        (1.0, date.old),
    ]
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // test assertions use exact f32 literals we control
mod test {
    use super::*;

    fn rgb(r: u8, g: u8, b: u8) -> Style {
        Style::from(Color::Rgb(r, g, b))
    }

    fn rgb_bold(r: u8, g: u8, b: u8) -> Style {
        Style::from(Color::Rgb(r, g, b)).bold()
    }

    /// The 6-anchor date layout used by most tests.  Colours are
    /// obviously synthetic (pure primaries on the hue wheel) so
    /// tests can inspect bytes directly without floating-point
    /// tolerances on interpolated midpoints.
    fn synthetic_date() -> [(f32, Style); 6] {
        [
            (0.0, rgb_bold(255, 0, 0)), // red, bold
            (0.2, rgb(255, 128, 0)),    // orange
            (0.4, rgb(255, 255, 0)),    // yellow
            (0.6, rgb(0, 255, 0)),      // green
            (0.8, rgb(0, 0, 255)),      // blue
            (1.0, rgb(128, 0, 128)),    // purple
        ]
    }

    fn fg(style: Style) -> (u8, u8, u8) {
        match style.foreground {
            Some(Color::Rgb(r, g, b)) => (r, g, b),
            _ => panic!("expected Rgb foreground, got {:?}", style.foreground),
        }
    }

    #[test]
    fn lut_is_full_length() {
        let lut = build_smooth_lut(&synthetic_date());
        assert_eq!(lut.len(), LUT_SIZE);
    }

    #[test]
    fn first_stop_matches_first_anchor_exactly() {
        let anchors = synthetic_date();
        let lut = build_smooth_lut(&anchors);

        // Same foreground...
        assert_eq!(fg(lut[0]), fg(anchors[0].1));
        // ...and same bold attribute.
        assert!(
            lut[0].is_bold,
            "first stop should inherit bold from anchor[0]"
        );
    }

    #[test]
    fn last_stop_matches_last_anchor_exactly() {
        let anchors = synthetic_date();
        let lut = build_smooth_lut(&anchors);

        assert_eq!(fg(lut[LUT_SIZE - 1]), fg(anchors[5].1));
    }

    /// For 6 evenly-spaced anchors in a 256-stop LUT, interior
    /// anchors land on exact integer bucket indices: 0, 51, 102,
    /// 153, 204, 255.  At those positions the LUT should return
    /// the anchor's own foreground exactly (no Oklab round-trip
    /// drift).
    #[test]
    fn interior_anchors_land_on_their_own_buckets() {
        let anchors = synthetic_date();
        let lut = build_smooth_lut(&anchors);

        let expected_buckets = [
            (0, anchors[0].1),
            (51, anchors[1].1),
            (102, anchors[2].1),
            (153, anchors[3].1),
            (204, anchors[4].1),
            (255, anchors[5].1),
        ];

        for (bucket, anchor_style) in expected_buckets {
            let (ar, ag, ab) = fg(anchor_style);
            let (lr, lg, lb) = fg(lut[bucket]);

            // sRGB↔Oklab round-trip preservation is ±1 per
            // channel (see oklab.rs round_trip_preservation),
            // but when t is exactly 0 the lerp returns the
            // first anchor unchanged.  Allow ±1 to cover the
            // case where a round-trip does happen.
            assert!(
                (i32::from(ar) - i32::from(lr)).abs() <= 1
                    && (i32::from(ag) - i32::from(lg)).abs() <= 1
                    && (i32::from(ab) - i32::from(lb)).abs() <= 1,
                "bucket {bucket}: expected {:?}, got {:?}",
                (ar, ag, ab),
                (lr, lg, lb),
            );
        }
    }

    /// Bias-toward-hotter: every stop between anchor[0] and
    /// anchor[1] should carry anchor[0]'s bold attribute.
    #[test]
    fn interpolated_stops_inherit_earlier_anchor_attributes() {
        let anchors = synthetic_date();
        let lut = build_smooth_lut(&anchors);

        // anchor[0] is bold, anchor[1] is not.  Buckets 1..=50
        // sit strictly between them.
        for bucket in 1..=50 {
            assert!(
                lut[bucket].is_bold,
                "bucket {bucket} should be bold (bias-toward-hotter from anchor[0])",
            );
        }
        // Bucket 51 is anchor[1] exactly — not bold.
        assert!(!lut[51].is_bold, "bucket 51 (anchor[1]) should not be bold");
    }

    /// The red→orange segment should see the green channel
    /// climb monotonically from 0 (anchor[0]) to 128 (anchor[1])
    /// — a sanity check that Oklab interpolation is producing
    /// sensible in-betweens rather than dead grey.
    #[test]
    fn interpolation_produces_monotonic_midpoints() {
        let anchors = synthetic_date();
        let lut = build_smooth_lut(&anchors);

        let g_channel: Vec<u8> = (0..=51).map(|i| fg(lut[i]).1).collect();

        for window in g_channel.windows(2) {
            assert!(
                window[0] <= window[1],
                "green channel not monotonic across red→orange: {g_channel:?}",
            );
        }
        // And it actually moves a meaningful amount.
        assert!(
            g_channel[51] > g_channel[0] + 20,
            "green channel barely moved: {} → {}",
            g_channel[0],
            g_channel[51],
        );
    }

    #[test]
    fn size_anchors_layout_matches_five_tiers() {
        let size = Size {
            number_byte: rgb(0x11, 0x11, 0x11),
            number_kilo: rgb(0x22, 0x22, 0x22),
            number_mega: rgb(0x44, 0x44, 0x44),
            number_giga: rgb(0x88, 0x88, 0x88),
            number_huge: rgb(0xCC, 0xCC, 0xCC),
            ..Size::default()
        };

        let anchors = size_anchors(&size);
        assert_eq!(anchors.len(), 5);
        assert_eq!(anchors[0].0, 0.00);
        assert_eq!(anchors[1].0, 0.25);
        assert_eq!(anchors[2].0, 0.50);
        assert_eq!(anchors[3].0, 0.75);
        assert_eq!(anchors[4].0, 1.00);

        assert_eq!(fg(anchors[0].1), (0x11, 0x11, 0x11));
        assert_eq!(fg(anchors[4].1), (0xCC, 0xCC, 0xCC));
    }

    #[test]
    fn date_anchors_layout_matches_six_tiers() {
        let date = DateAge {
            now: rgb(0x10, 0x10, 0x10),
            today: rgb(0x20, 0x20, 0x20),
            week: rgb(0x30, 0x30, 0x30),
            month: rgb(0x40, 0x40, 0x40),
            year: rgb(0x50, 0x50, 0x50),
            old: rgb(0x60, 0x60, 0x60),
            flat: Style::default(),
        };

        let anchors = date_anchors(&date);
        assert_eq!(anchors.len(), 6);
        assert_eq!(anchors[0].0, 0.0);
        assert_eq!(anchors[5].0, 1.0);

        assert_eq!(fg(anchors[0].1), (0x10, 0x10, 0x10));
        assert_eq!(fg(anchors[5].1), (0x60, 0x60, 0x60));
    }

    mod position_mapping {
        use super::*;

        /// Absolute tolerance for position comparisons — large
        /// enough to swallow f32 log rounding, small enough that a
        /// tier-boundary error would still fail the test (each
        /// segment is 0.2 wide).
        const EPS: f32 = 0.001;

        fn close(actual: f32, expected: f32) -> bool {
            (actual - expected).abs() <= EPS
        }

        #[test]
        fn size_boundary_values() {
            assert!(close(size_to_position(0), 0.0));
            assert!(close(size_to_position(1), 0.0));
            assert!(close(size_to_position(1024), 0.25));
            assert!(close(size_to_position(1024 * 1024), 0.5));
            assert!(close(size_to_position(1024 * 1024 * 1024), 0.75));
            assert!(close(size_to_position(1_u64 << 40), 1.0));
        }

        #[test]
        fn size_clamps_beyond_huge_anchor() {
            assert_eq!(size_to_position(1_u64 << 41), 1.0);
            assert_eq!(size_to_position(1_u64 << 50), 1.0);
            assert_eq!(size_to_position(u64::MAX), 1.0);
        }

        #[test]
        fn size_is_monotonic() {
            let samples: Vec<f32> = (0..=40).map(|i| size_to_position(1_u64 << i)).collect();
            for window in samples.windows(2) {
                assert!(
                    window[0] <= window[1],
                    "size_to_position not monotonic: {samples:?}",
                );
            }
        }

        #[test]
        fn size_log_midpoints_land_at_segment_centres() {
            // Halfway between byte (1) and kilo (1024) on a log
            // scale is 32 (= 2^5 = halfway in log₂ between 2^0 and
            // 2^10).  Its position should be ~0.125 (half of 0.25).
            assert!(close(size_to_position(32), 0.125));
            // And between kilo and mega: 32 KiB = 2^15 → position
            // halfway between 0.25 and 0.5 = 0.375.
            assert!(close(size_to_position(32 * 1024), 0.375));
        }

        #[test]
        fn age_boundary_values() {
            assert!(close(age_to_position(0), 0.0));
            assert!(close(age_to_position(1), 0.0));
            assert!(close(age_to_position(3_600), 0.2));
            assert!(close(age_to_position(86_400), 0.4));
            assert!(close(age_to_position(604_800), 0.6));
            assert!(close(age_to_position(2_592_000), 0.8));
            assert!(close(age_to_position(31_536_000), 1.0));
        }

        #[test]
        fn age_clamps_beyond_old_anchor() {
            assert_eq!(age_to_position(31_536_001), 1.0);
            assert_eq!(age_to_position(10_u64 * 31_536_000), 1.0);
            assert_eq!(age_to_position(u64::MAX), 1.0);
        }

        #[test]
        fn age_is_monotonic() {
            // Log-ish sweep: powers of 2 from 1 sec to ~1 year.
            let samples: Vec<f32> = (0..=25).map(|i| age_to_position(1_u64 << i)).collect();
            for window in samples.windows(2) {
                assert!(
                    window[0] <= window[1],
                    "age_to_position not monotonic: {samples:?}",
                );
            }
        }

        /// Geometric midpoint of the first segment: 60 seconds is
        /// halfway between 1 sec and 3600 sec on a log₂ scale
        /// (2^0 → 2^~11.81, midpoint at 2^~5.9 ≈ 59.6).  Its
        /// position should be very close to 0.1, the midpoint of
        /// the [0.0, 0.2] segment.
        #[test]
        fn age_log_midpoint_of_first_segment() {
            assert!(
                close(age_to_position(60), 0.1),
                "age_to_position(60) = {}",
                age_to_position(60),
            );
        }

        /// 1 week is at position 0.6 (the `month` anchor), 1 day
        /// at 0.4 (the `week` anchor).  The geometric midpoint of
        /// that segment is √(86400 × 604800) ≈ 228552 s ≈ 2.645
        /// days, which should sit exactly at position 0.5.
        #[test]
        fn age_log_midpoint_of_week_month_segment() {
            let geometric_midpoint = ((86_400_f64 * 604_800_f64).sqrt()) as u64;
            let pos = age_to_position(geometric_midpoint);
            assert!(
                close(pos, 0.5),
                "log midpoint ({geometric_midpoint}s) → position {pos}, expected ~0.5",
            );
        }
    }

    /// End-to-end integration: run the smooth-gradient build
    /// through `UiStyles::apply_gradient_flags` and check that
    /// the LUT storage on `UiStyles::smooth_luts` is populated
    /// only for the columns that meet the gate.
    mod apply_gradient_flags {
        use super::*;
        use crate::theme::GradientFlags;
        use crate::theme::ui_styles::UiStyles;

        fn ui_with_24bit_date_gradient() -> UiStyles {
            let mut ui = UiStyles::default();
            let date = DateAge {
                now: rgb_bold(0x3D, 0xD7, 0xD7),
                today: rgb(0x3D, 0xD7, 0xD7),
                week: rgb(0x3A, 0xAB, 0xAE),
                month: rgb(0x3B, 0x8E, 0xD8),
                year: rgb(0x88, 0x88, 0x88),
                old: rgb(0x5C, 0x5C, 0x5C),
                flat: rgb(0x55, 0x55, 0x55),
            };
            ui.date_modified = date;
            ui.date_accessed = date;
            ui.date_changed = date;
            ui.date_created = date;
            ui.size = Size {
                major: rgb(0xAA, 0xAA, 0xAA),
                minor: rgb(0x55, 0x55, 0x55),
                number_byte: rgb(0x30, 0x60, 0xC0),
                number_kilo: rgb(0x40, 0x80, 0xA0),
                number_mega: rgb(0x60, 0xA0, 0x80),
                number_giga: rgb(0x80, 0xC0, 0x60),
                number_huge: rgb(0xA0, 0xE0, 0x40),
                unit_byte: rgb(0x55, 0x55, 0x55),
                unit_kilo: rgb(0x55, 0x55, 0x55),
                unit_mega: rgb(0x55, 0x55, 0x55),
                unit_giga: rgb(0x55, 0x55, 0x55),
                unit_huge: rgb(0x55, 0x55, 0x55),
            };
            ui
        }

        #[test]
        fn smooth_off_leaves_every_lut_none() {
            let mut ui = ui_with_24bit_date_gradient();
            // GradientFlags::ALL has smooth = true since 0.10;
            // turn it off explicitly for this test.
            let mut gradient = GradientFlags::ALL;
            gradient.smooth = false;
            ui.apply_gradient_flags(gradient);

            assert!(ui.smooth_luts.size.is_none());
            assert!(ui.smooth_luts.modified.is_none());
            assert!(ui.smooth_luts.accessed.is_none());
            assert!(ui.smooth_luts.changed.is_none());
            assert!(ui.smooth_luts.created.is_none());
        }

        #[test]
        fn smooth_on_builds_all_five_luts_when_anchors_are_rgb() {
            let mut ui = ui_with_24bit_date_gradient();
            let mut gradient = GradientFlags::ALL;
            gradient.smooth = true;
            ui.apply_gradient_flags(gradient);

            assert!(ui.smooth_luts.size.is_some(), "size LUT should be built");
            assert!(
                ui.smooth_luts.modified.is_some(),
                "modified LUT should be built"
            );
            assert!(
                ui.smooth_luts.accessed.is_some(),
                "accessed LUT should be built"
            );
            assert!(
                ui.smooth_luts.changed.is_some(),
                "changed LUT should be built"
            );
            assert!(
                ui.smooth_luts.created.is_some(),
                "created LUT should be built"
            );
        }

        #[test]
        fn smooth_on_skips_columns_with_gradient_off() {
            let mut ui = ui_with_24bit_date_gradient();
            let mut gradient = GradientFlags::ALL;
            gradient.smooth = true;
            gradient.modified = false; // modified has gradient off
            ui.apply_gradient_flags(gradient);

            assert!(ui.smooth_luts.size.is_some());
            assert!(
                ui.smooth_luts.modified.is_none(),
                "modified LUT should be skipped"
            );
            assert!(ui.smooth_luts.accessed.is_some());
            assert!(ui.smooth_luts.changed.is_some());
            assert!(ui.smooth_luts.created.is_some());
        }

        #[test]
        fn smooth_on_skips_columns_that_are_not_smoothable() {
            let mut ui = ui_with_24bit_date_gradient();
            // Swap the size's byte anchor for a palette colour —
            // the whole size column should drop out of smoothing.
            ui.size.number_byte = Style::from(Color::Fixed(196));

            let mut gradient = GradientFlags::ALL;
            gradient.smooth = true;
            ui.apply_gradient_flags(gradient);

            assert!(
                ui.smooth_luts.size.is_none(),
                "size LUT skipped: non-RGB anchor"
            );
            // Timestamp columns were untouched and should still build.
            assert!(ui.smooth_luts.modified.is_some());
        }

        /// Flattening runs *after* LUT build, so a smooth build
        /// that passed the gate still sees the original per-tier
        /// colours — flattening doesn't retroactively disable it.
        #[test]
        fn smooth_build_sees_pre_flatten_anchors() {
            let mut ui = ui_with_24bit_date_gradient();
            let mut gradient = GradientFlags::ALL;
            gradient.smooth = true;
            ui.apply_gradient_flags(gradient);

            // The built LUT's first stop should match the
            // original `now` anchor, even though the non-smoothed
            // tier fields would be unchanged in this case anyway.
            let lut = ui.smooth_luts.modified.as_ref().expect("LUT was built");
            let Some(Color::Rgb(r, g, b)) = lut[0].foreground else {
                panic!("LUT[0] should have Rgb foreground");
            };
            assert!(
                (i32::from(r) - 0x3D).abs() <= 1
                    && (i32::from(g) - 0xD7).abs() <= 1
                    && (i32::from(b) - 0xD7).abs() <= 1,
                "LUT[0] = ({r}, {g}, {b}), expected ~(0x3D, 0xD7, 0xD7)",
            );
        }
    }

    /// Building a size LUT from all-RGB anchors and reading it
    /// at its five exact tier positions should return the tier
    /// colours (within the ±1 Oklab round-trip tolerance).
    #[test]
    fn build_lut_from_size_anchors_round_trip() {
        let size = Size {
            number_byte: rgb(0x30, 0x60, 0xC0),
            number_kilo: rgb(0x40, 0x80, 0xA0),
            number_mega: rgb(0x60, 0xA0, 0x80),
            number_giga: rgb(0x80, 0xC0, 0x60),
            number_huge: rgb(0xA0, 0xE0, 0x40),
            ..Size::default()
        };

        let lut = build_smooth_lut(&size_anchors(&size));

        // 5 anchors → buckets 0, 63 or 64, 127 or 128, 191 or 192, 255.
        // The exact positions are 0.00, 0.25, 0.50, 0.75, 1.00,
        // which as LUT indices (position * 255) are
        // 0, 63.75, 127.5, 191.25, 255.
        // Due to rounding we accept the closest integer bucket.
        let samples = [
            (0, size.number_byte),
            (64, size.number_kilo),  // round(63.75) = 64
            (128, size.number_mega), // round(127.5) = 128
            (191, size.number_giga), // round(191.25) = 191
            (255, size.number_huge),
        ];

        for (bucket, expected) in samples {
            let (er, eg, eb) = fg(expected);
            let (lr, lg, lb) = fg(lut[bucket]);

            // Allow ±4 per channel: interior anchors don't land
            // *exactly* on integer bucket indices for 5 anchors
            // in a 256-stop LUT (e.g. anchor[1] is at position
            // 0.25 = bucket 63.75, so bucket 64 is slightly past
            // the anchor and we get a tiny bit of interpolation
            // toward anchor[2]).
            assert!(
                (i32::from(er) - i32::from(lr)).abs() <= 4
                    && (i32::from(eg) - i32::from(lg)).abs() <= 4
                    && (i32::from(eb) - i32::from(lb)).abs() <= 4,
                "bucket {bucket}: expected {:?}, got {:?}",
                (er, eg, eb),
                (lr, lg, lb),
            );
        }
    }
}
