use glam::FloatExt;
use image::Rgba;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Sub};

/// Single precision color with Red, Green, Blue and Alpha channels.
///
/// Default color is [Color::BLACK].
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Color(pub [f32; 4]);

impl Color {
    pub const RED: Self = Self::new_rgb(1.0, 0.0, 0.0);
    pub const ORANGE: Self = Self::new_rgb(1.0, 0.5, 0.0);
    pub const YELLOW: Self = Self::new_rgb(1.0, 1.0, 0.0);
    pub const GREEN: Self = Self::new_rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::new_rgb(0.0, 0.0, 1.0);
    pub const CYAN: Self = Self::new_rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::new_rgb(1.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new_rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new_rgb(0.0, 0.0, 0.0);
    pub const BLACK_TRANSPARENT: Self = Self::new_rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn new_rgb(r: f32, g: f32, b: f32) -> Self {
        Self([r, g, b, 1.0])
    }

    pub const fn new_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    pub const fn new_gray(value: f32) -> Self {
        Self([value, value, value, 1.0])
    }

    pub const fn from_hex_rgb(argb: u32) -> Self {
        Self([
            ((argb & 0x00ff0000) >> 16) as f32 / 255.0,
            ((argb & 0x0000ff00) >> 8) as f32 / 255.0,
            (argb & 0x000000ff) as f32 / 255.0,
            1.0,
        ])
    }

    /// Creates a new color from hue, saturation, luminance and alpha values.
    /// Hue `h` is normalized to 1.0 (0.0 is 0°, 1.0 is 360°).
    pub const fn new_hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let h = h % 1.0;
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
        let (r, g, b) = if h < 1.0 / 6.0 {
            (c, x, 0.0)
        } else if h < 2.0 / 6.0 {
            (x, c, 0.0)
        } else if h < 3.0 / 6.0 {
            (0.0, c, x)
        } else if h < 4.0 / 6.0 {
            (0.0, x, c)
        } else if h < 5.0 / 6.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };
        let m = l - c / 2.0;
        Self([r + m, g + m, b + m, a])
    }

    /// Creates a new color from hue, saturation, and luminance values.
    /// Hue `h` is normalized to 1.0 (0.0 is 0°, 1.0 is 360°).
    pub const fn new_hsl(h: f32, s: f32, l: f32) -> Self {
        Self::new_hsla(h, s, l, 1.0)
    }

    /// Builds a color with given alpha value.
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.0[3] = a;
        self
    }

    /// Returns linear interpolation of `self` with `other` color. When `a` is 0.0, result is
    /// identical to `self`, when `a` is 1.0, result is identical to `other`.
    pub fn lerp(&self, other: &Self, a: f32) -> Self {
        Self([
            self.0[0].lerp(other.0[0], a),
            self.0[1].lerp(other.0[1], a),
            self.0[2].lerp(other.0[2], a),
            self.0[3].lerp(other.0[3], a),
        ])
    }

    /// Returns a `Color` where each component is the max of the two operands components.
    pub fn max(&self, other: &Self) -> Self {
        Self([
            self.0[0].max(other.0[0]),
            self.0[1].max(other.0[1]),
            self.0[2].max(other.0[2]),
            self.0[3].max(other.0[3]),
        ])
    }

    /// Convert to [`Rgba<u8>`] color.
    pub fn as_rgba8(&self) -> Rgba<u8> {
        Rgba::<u8>(self.0.map(|x| (x * 255.0) as u8))
    }

    /// Convert to [`Rgba<f32>`] color.
    pub fn as_rgbaf32(&self) -> Rgba<f32> {
        Rgba::<f32>(self.0)
    }

    /// Returns alpha channel value.
    pub fn alpha(&self) -> f32 {
        self.0[3]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl From<Rgba<u8>> for Color {
    fn from(value: Rgba<u8>) -> Self {
        Self(value.0.map(|x| (x as f32) / 255.0))
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
        ])
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..self.0.len() {
            self.0[i] += rhs.0[i]
        }
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
            self.0[3] - rhs.0[3],
        ])
    }
}

impl Div<f32> for Color {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self([
            self.0[0] / rhs,
            self.0[1] / rhs,
            self.0[2] / rhs,
            self.0[3] / rhs,
        ])
    }
}

impl DivAssign<f32> for Color {
    fn div_assign(&mut self, rhs: f32) {
        for i in 0..self.0.len() {
            self.0[i] /= rhs
        }
    }
}
