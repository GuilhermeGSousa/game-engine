use crate::LinearRgba;

/// Hue-Saturation-Lightness + alpha, all in [0.0, 1.0].
///
/// Hue is normalised to [0.0, 1.0) rather than [0°, 360°).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsl {
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub a: f32,
}

impl Hsl {
    #[inline]
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Hue in degrees [0°, 360°).
    #[inline]
    pub fn hue_degrees(&self) -> f32 {
        self.h * 360.0
    }

    #[inline]
    pub fn to_linear(self) -> LinearRgba {
        LinearRgba::from(self)
    }
}

impl From<LinearRgba> for Hsl {
    fn from(c: LinearRgba) -> Self {
        let max = c.r.max(c.g).max(c.b);
        let min = c.r.min(c.g).min(c.b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        let h = if delta == 0.0 {
            0.0
        } else if max == c.r {
            ((c.g - c.b) / delta).rem_euclid(6.0) / 6.0
        } else if max == c.g {
            ((c.b - c.r) / delta + 2.0) / 6.0
        } else {
            ((c.r - c.g) / delta + 4.0) / 6.0
        };

        Self::new(h, s, l, c.a)
    }
}

impl From<Hsl> for LinearRgba {
    fn from(c: Hsl) -> Self {
        if c.s == 0.0 {
            return LinearRgba::new(c.l, c.l, c.l, c.a);
        }

        let chroma = (1.0 - (2.0 * c.l - 1.0).abs()) * c.s;
        let h6 = c.h * 6.0;
        let x = chroma * (1.0 - (h6 % 2.0 - 1.0).abs());
        let m = c.l - chroma / 2.0;

        let (r1, g1, b1) = match h6 as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };

        LinearRgba::new(r1 + m, g1 + m, b1 + m, c.a)
    }
}
