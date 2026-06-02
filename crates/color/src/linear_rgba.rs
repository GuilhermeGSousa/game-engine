use bytemuck::{Pod, Zeroable};

use crate::{Hsl, Srgba};

/// Linear RGBA color, suitable for use as a GPU uniform.
///
/// Values are in the range [0.0, 1.0] in linear light.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct LinearRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl LinearRgba {
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from raw RGBA bytes (0–255), interpreting them as linear values.
    #[inline]
    pub fn from_bytes(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Construct from a packed `u32` in RGBA byte order.
    #[inline]
    pub fn from_rgba_u32(packed: u32) -> Self {
        let [r, g, b, a] = packed.to_be_bytes();
        Self::from_bytes(r, g, b, a)
    }

    /// Relative luminance (ITU-R BT.709), valid for linear light.
    #[inline]
    pub fn luminance(&self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Return as `[f32; 4]` for use where a raw array is needed.
    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[inline]
    pub fn to_srgba(self) -> Srgba {
        Srgba::from(self)
    }

    #[inline]
    pub fn to_hsl(self) -> Hsl {
        Hsl::from(self)
    }
}

impl From<[f32; 4]> for LinearRgba {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<LinearRgba> for [f32; 4] {
    fn from(c: LinearRgba) -> Self {
        c.to_array()
    }
}
