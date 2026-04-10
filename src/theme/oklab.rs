//! sRGB ↔ Oklab colour-space conversion.
//!
//! Used by the smooth-gradient feature to interpolate between
//! theme anchor colours in a perceptually uniform space, so
//! that intermediate stops look like natural in-betweens rather
//! than muddy midpoints.
//!
//! The conversion formulas — including the M1 and M2 matrices,
//! the cube-root nonlinearity, and the sRGB transfer function —
//! are taken directly from Björn Ottosson's original 2020
//! article:
//!
//!   Björn Ottosson, *"A perceptual color space for image
//!   processing"*, 2020.
//!   <https://bottosson.github.io/posts/oklab/>
//!
//! The matrix constants are mathematics, not code; this module
//! is a fresh implementation of the published formulas with no
//! code copied from any other source.

// Single-character bindings (r/g/b, L/a/b, l/m/s, l_/m_/s_)
// come straight from Ottosson's paper; renaming them would
// make the code harder to cross-reference with the source.
#![allow(clippy::many_single_char_names)]

/// Convert an sRGB triplet (each channel in 0..=255) to Oklab
/// `(L, a, b)`.
///
/// `L` is perceived lightness in [0, 1]; `a` and `b` are
/// opponent-colour axes, each roughly in [-0.4, 0.4] for
/// in-gamut sRGB colours.
pub fn srgb_to_oklab(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = srgb_to_linear(f32::from(r) / 255.0);
    let g = srgb_to_linear(f32::from(g) / 255.0);
    let b = srgb_to_linear(f32::from(b) / 255.0);

    // Linear sRGB → LMS (Ottosson's M1).
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5   * r + 0.680_699_5  * g + 0.107_396_96  * b;
    let s = 0.088_302_46  * r + 0.281_718_85 * g + 0.629_978_7   * b;

    // Nonlinearity: cube root.
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // L′M′S′ → Oklab (Ottosson's M2).
    let oklab_l = 0.210_454_26  * l_ + 0.793_617_8   * m_ - 0.004_072_047 * s_;
    let oklab_a = 1.977_998_5   * l_ - 2.428_592_2   * m_ + 0.450_593_7   * s_;
    let oklab_b = 0.025_904_037 * l_ + 0.782_771_77  * m_ - 0.808_675_77  * s_;

    (oklab_l, oklab_a, oklab_b)
}

/// Convert an Oklab `(L, a, b)` colour back to an sRGB triplet
/// (each channel clamped to 0..=255).
pub fn oklab_to_srgb(l: f32, a: f32, b: f32) -> (u8, u8, u8) {
    // Oklab → L′M′S′ (inverse M2).
    let l_ = l + 0.396_337_78  * a + 0.215_803_76  * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17  * b;
    let s_ = l - 0.089_484_18  * a - 1.291_485_5   * b;

    // Undo the cube-root nonlinearity.
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // LMS → linear sRGB (inverse M1).
    let r =  4.076_741_7   * l - 3.307_711_6   * m + 0.230_969_94  * s;
    let g = -1.268_438     * l + 2.609_757_4   * m - 0.341_319_38  * s;
    let b = -0.004_196_086 * l - 0.703_418_6   * m + 1.707_614_7   * s;

    (
        to_srgb_byte(linear_to_srgb(r)),
        to_srgb_byte(linear_to_srgb(g)),
        to_srgb_byte(linear_to_srgb(b)),
    )
}

/// Interpolate between two sRGB colours in Oklab space.
///
/// `t` is clamped to `[0.0, 1.0]`: `t = 0.0` returns `c1`,
/// `t = 1.0` returns `c2`, and intermediate values return the
/// Oklab-lerped midpoint.
pub fn lerp_oklab(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);

    let (l1, a1, b1) = srgb_to_oklab(c1.0, c1.1, c1.2);
    let (l2, a2, b2) = srgb_to_oklab(c2.0, c2.1, c2.2);

    let l = l1 + (l2 - l1) * t;
    let a = a1 + (a2 - a1) * t;
    let b = b1 + (b2 - b1) * t;

    oklab_to_srgb(l, a, b)
}

/// sRGB transfer function (gamma decode) applied to one
/// channel in `[0, 1]`.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Inverse sRGB transfer function (gamma encode) applied to
/// one channel in `[0, 1]`.
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Clamp a linear-sRGB channel in `[0, 1]` to the nearest
/// `u8` value.
fn to_srgb_byte(c: f32) -> u8 {
    (c * 255.0).round().clamp(0.0, 255.0) as u8
}


#[cfg(test)]
mod test {
    use super::*;

    /// Absolute tolerance for Oklab axis comparisons against
    /// Ottosson's published reference values.
    const OKLAB_EPS: f32 = 0.001;

    fn close(actual: f32, expected: f32, eps: f32) -> bool {
        (actual - expected).abs() <= eps
    }

    #[test]
    fn white_reference_value() {
        let (l, a, b) = srgb_to_oklab(255, 255, 255);
        assert!(close(l, 1.000, OKLAB_EPS), "L was {l}");
        assert!(close(a, 0.000, OKLAB_EPS), "a was {a}");
        assert!(close(b, 0.000, OKLAB_EPS), "b was {b}");
    }

    #[test]
    fn black_reference_value() {
        let (l, a, b) = srgb_to_oklab(0, 0, 0);
        assert!(close(l, 0.000, OKLAB_EPS), "L was {l}");
        assert!(close(a, 0.000, OKLAB_EPS), "a was {a}");
        assert!(close(b, 0.000, OKLAB_EPS), "b was {b}");
    }

    #[test]
    fn red_reference_value() {
        // Ottosson's reference: pure sRGB red → L≈0.628, a≈0.225, b≈0.126
        let (l, a, b) = srgb_to_oklab(255, 0, 0);
        assert!(close(l, 0.628, OKLAB_EPS), "L was {l}");
        assert!(close(a, 0.225, OKLAB_EPS), "a was {a}");
        assert!(close(b, 0.126, OKLAB_EPS), "b was {b}");
    }

    #[test]
    fn green_reference_value() {
        // Ottosson's reference: pure sRGB green → L≈0.866, a≈-0.234, b≈0.180
        let (l, a, b) = srgb_to_oklab(0, 255, 0);
        assert!(close(l, 0.866, OKLAB_EPS), "L was {l}");
        assert!(close(a, -0.234, OKLAB_EPS), "a was {a}");
        assert!(close(b, 0.180, OKLAB_EPS), "b was {b}");
    }

    #[test]
    fn blue_reference_value() {
        // Ottosson's reference: pure sRGB blue → L≈0.452, a≈-0.032, b≈-0.312
        let (l, a, b) = srgb_to_oklab(0, 0, 255);
        assert!(close(l, 0.452, OKLAB_EPS), "L was {l}");
        assert!(close(a, -0.032, OKLAB_EPS), "a was {a}");
        assert!(close(b, -0.312, OKLAB_EPS), "b was {b}");
    }

    /// Round-trip preservation: converting sRGB → Oklab → sRGB
    /// should return the original triplet within ±1 per channel
    /// on a sweep of the colour cube.
    #[test]
    fn round_trip_preservation() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    let (l, a_, b_) = srgb_to_oklab(r, g, b);
                    let (r2, g2, b2) = oklab_to_srgb(l, a_, b_);

                    let dr = i32::from(r) - i32::from(r2);
                    let dg = i32::from(g) - i32::from(g2);
                    let db = i32::from(b) - i32::from(b2);

                    assert!(
                        dr.abs() <= 1 && dg.abs() <= 1 && db.abs() <= 1,
                        "round-trip failed: ({r},{g},{b}) → ({r2},{g2},{b2})",
                    );
                }
            }
        }
    }

    #[test]
    fn lerp_endpoints_match_inputs() {
        let red = (255, 0, 0);
        let blue = (0, 0, 255);

        let start = lerp_oklab(red, blue, 0.0);
        let end = lerp_oklab(red, blue, 1.0);

        // Endpoints should round-trip within the ±1 per-channel
        // tolerance we've already established.
        assert!(
            (i32::from(start.0) - 255).abs() <= 1 && start.1 == 0 && start.2 == 0,
            "start was {start:?}",
        );
        assert!(
            end.0 == 0 && end.1 == 0 && (i32::from(end.2) - 255).abs() <= 1,
            "end was {end:?}",
        );
    }

    #[test]
    fn lerp_t_is_clamped() {
        let red = (255, 0, 0);
        let blue = (0, 0, 255);

        // Out-of-range t should behave as if clamped to [0, 1].
        assert_eq!(lerp_oklab(red, blue, -5.0), lerp_oklab(red, blue, 0.0));
        assert_eq!(lerp_oklab(red, blue, 5.0), lerp_oklab(red, blue, 1.0));
    }

    /// Midpoint of red→green should cross through yellow-ish
    /// territory, not dirty grey — this is the whole point of
    /// interpolating in Oklab rather than linear sRGB.  The
    /// specific target values here aren't Ottosson references,
    /// just a sanity check that the R and G channels are both
    /// present and comparable at the midpoint.
    #[test]
    fn red_green_midpoint_is_not_muddy() {
        let midpoint = lerp_oklab((255, 0, 0), (0, 255, 0), 0.5);
        let (r, g, b) = midpoint;

        // Both R and G should be present.
        assert!(r > 100, "R channel too low at midpoint: {midpoint:?}");
        assert!(g > 100, "G channel too low at midpoint: {midpoint:?}");
        // B should still be close to zero — red↔green carries no blue.
        assert!(b < 50, "B channel unexpectedly high at midpoint: {midpoint:?}");
    }
}
