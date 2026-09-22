use crate::rect::{DRect, Rect};
use glam::{DVec2, Vec2, dvec2, vec2};
use std::ops::Index;

/// Defines a quad from 4 corners coordinates of `f32` precision.
///
/// A [Quad] can be constructed from a [Rect]. The coordinates are in this order:
/// [(x1, y1), (x1, y2), (x2, y2), (x2, y1)].
///
/// Conversion from a [Quad] to a [Rect] is however not possible.
pub struct Quad(pub [Vec2; 4]);

impl Quad {
    /// Creates a quad from two corner coordinates.
    pub fn from_corners(x1y1: Vec2, x2y2: Vec2) -> Self {
        Self([x1y1, vec2(x1y1.x, x2y2.y), x2y2, vec2(x2y2.x, x1y1.y)])
    }
}

impl From<Rect> for Quad {
    fn from(value: Rect) -> Self {
        Self([value.x1y1(), value.x1y2(), value.x2y2(), value.x2y1()])
    }
}

impl Index<usize> for Quad {
    type Output = Vec2;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Defines a quad from 4 corners coordinates of `f64` precision.
///
/// A [DQuad] can be constructed from a [DRect]. The coordinates are in this order:
/// [(x1, y1), (x1, y2), (x2, y2), (x2, y1)].
///
/// Conversion from a [DQuad] to a [DRect] is however not possible.
pub struct DQuad(pub [DVec2; 4]);

impl DQuad {
    /// Creates a quad from two corner coordinates.
    pub fn from_corners(x1y1: DVec2, x2y2: DVec2) -> Self {
        Self([x1y1, dvec2(x1y1.x, x2y2.y), x2y2, dvec2(x2y2.x, x1y1.y)])
    }
}

impl From<DRect> for DQuad {
    fn from(value: DRect) -> Self {
        Self([value.x1y1(), value.x1y2(), value.x2y2(), value.x2y1()])
    }
}

impl Index<usize> for DQuad {
    type Output = DVec2;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
