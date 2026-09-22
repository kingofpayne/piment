use crate::{
    axis::Axis,
    quad::{DQuad, Quad},
};
use glam::{DVec2, FloatExt, Vec2, dvec2, vec2};
use std::ops::Add;

/// Oriented rectangle defined by `f32` coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Abscissa of first corner.
    pub x1: f32,
    /// Ordinate of first corner.
    pub y1: f32,
    /// Abscissa of second corner.
    pub x2: f32,
    /// Ordinate of second corner.
    pub y2: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect::new(0.0, 0.0, 0.0, 0.0);

    /// Constructs a new rectangle.
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Constructs a new rectangle of null area at given position.
    pub const fn from_pos(pos: Vec2) -> Self {
        Self::new(pos.x, pos.y, pos.x, pos.y)
    }

    /// Constructs a new rectangle from a given position and size.
    /// `pos` is the position of the first corner, and second corner is at `pos + size`.
    pub const fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self::new(pos.x, pos.y, pos.x + size.x, pos.y + size.y)
    }

    /// Constructs a new rectangle from a given center and size.
    pub const fn from_center_size(pos: Vec2, size: Vec2) -> Self {
        Self::new(
            pos.x - size.x / 2.0,
            pos.y - size.y / 2.0,
            pos.x + size.x / 2.0,
            pos.y + size.y / 2.0,
        )
    }

    /// Constructs a rectangle from two corner coordinates.
    pub fn from_corners(x1y1: Vec2, x2y2: Vec2) -> Self {
        Self {
            x1: x1y1.x,
            y1: x1y1.y,
            x2: x2y2.x,
            y2: x2y2.y,
        }
    }

    /// Returns center of the rectangle.
    pub fn center(&self) -> Vec2 {
        Vec2 {
            x: self.x1.midpoint(self.x2),
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns left anchor of the rectangle.
    pub fn left(&self) -> Vec2 {
        Vec2 {
            x: self.x1,
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns right anchor of the rectangle.
    pub fn right(&self) -> Vec2 {
        Vec2 {
            x: self.x2,
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns top anchor of the rectangle.
    pub fn top(&self) -> Vec2 {
        vec2(self.x1.midpoint(self.x2), self.y1)
    }

    /// Returns bottom anchor of the rectangle.
    pub fn bottom(&self) -> Vec2 {
        vec2(self.x1.midpoint(self.x2), self.y2)
    }

    /// Returns horizontal center of the rectangle.
    pub fn h_center(&self) -> f32 {
        self.x1.midpoint(self.x2)
    }

    /// Returns vertical center of the rectangle.
    pub fn v_center(&self) -> f32 {
        self.y1.midpoint(self.y2)
    }

    /// Returns a point in the rectangle using linear interpolation of the edge coordinates.
    ///
    /// For instance, `lerp_xy(0, 0.5)` will return the center of the left edge.
    pub fn lerp_xy(&self, x: f32, y: f32) -> Vec2 {
        vec2(self.x1.lerp(self.x2, x), self.y1.lerp(self.y2, y))
    }

    /// Returns (x1, y1) corner coordinates as a vector.
    pub fn x1y1(&self) -> Vec2 {
        vec2(self.x1, self.y1)
    }

    /// Returns (x1, y2) corner coordinates as a vector.
    pub fn x1y2(&self) -> Vec2 {
        vec2(self.x1, self.y2)
    }

    /// Returns (x2, y1) corner coordinates as a vector.
    pub fn x2y1(&self) -> Vec2 {
        vec2(self.x2, self.y1)
    }

    /// Returns (x2, y2) corner coordinates as a vector.
    pub fn x2y2(&self) -> Vec2 {
        vec2(self.x2, self.y2)
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    /// Returns the area of the rectangle.
    pub fn area(&self) -> f32 {
        ((self.x2 - self.x1) * (self.y2 - self.y1)).abs()
    }

    /// Returns the oriented size of the rectangle `(x2 - x1, y2 - y1)`.
    pub fn size(&self) -> Vec2 {
        vec2(self.x2 - self.x1, self.y2 - self.y1)
    }

    /// Returns either width or height depending on selected `axis`.
    pub fn size_axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.width(),
            Axis::Y => self.height(),
        }
    }

    /// Returns the minimum rectangle containing `self` and `other` rectangles.
    pub fn union(&self, other: &Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    /// Returns the maximum rectangle contained by `self` and `other` rectangles.
    pub fn intersection(self, other: Self) -> Self {
        Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        }
    }

    /// Returns true if `self` and `other` rectangles have non-null intersection.
    /// Used for AABB collision testing for instance.
    pub fn intersects(&self, other: &Self) -> bool {
        let overlap_x = self.x1 < other.x2 && self.x2 > other.x1;
        let overlap_y = self.y1 < other.y2 && self.y2 > other.y1;
        overlap_x && overlap_y
    }

    /// Returns true if given point is inside the rectangle, bounds inclusive.
    pub fn contains(&self, point: Vec2) -> bool {
        (self.x1..=self.x2).contains(&point.x) && (self.y1..=self.y2).contains(&point.y)
    }

    /// Returns true if given abscissa is between the rectangle left and right bounds inclusive.
    pub fn contains_x(&self, x: f32) -> bool {
        (self.x1..=self.x2).contains(&x)
    }

    /// Returns true if given ordinate is between the rectangle top and bottom bounds inclusive.
    pub fn contains_y(&self, x: f32) -> bool {
        (self.y1..=self.y2).contains(&x)
    }

    /// Expands the rectangle by `amount` on each side. If `amount` is negative, the rectangle
    /// shrinks. Note that the result can have negative width.
    pub fn expand(&self, amount: f32) -> Self {
        Self {
            x1: self.x1 - amount,
            y1: self.y1 - amount,
            x2: self.x2 + amount,
            y2: self.y2 + amount,
        }
    }

    /// Returns the rectangle with rounded coordinates.
    pub fn round(self) -> Self {
        Self {
            x1: self.x1.round(),
            y1: self.y1.round(),
            x2: self.x2.round(),
            y2: self.y2.round(),
        }
    }

    /// Creates a quad from the rectangle.
    pub fn to_quad(self) -> Quad {
        self.into()
    }

    /// Returns the same rectangle but with `x1 <= x2` and `y1 < y2`.
    pub fn sorted(&self) -> Self {
        Self {
            x1: self.x1.min(self.x2),
            x2: self.x1.max(self.x2),
            y1: self.y1.min(self.y2),
            y2: self.y1.max(self.y2),
        }
    }
}

impl From<Vec2> for Rect {
    fn from(value: Vec2) -> Self {
        Self::from_pos(value)
    }
}

impl Add<Vec2> for Rect {
    type Output = Rect;

    fn add(self, rhs: Vec2) -> Self::Output {
        Rect {
            x1: self.x1 + rhs.x,
            y1: self.y1 + rhs.y,
            x2: self.x2 + rhs.x,
            y2: self.y2 + rhs.y,
        }
    }
}

impl Add<Rect> for Rect {
    type Output = Rect;

    fn add(self, rhs: Rect) -> Self::Output {
        Self {
            x1: self.x1 + rhs.x1,
            y1: self.y1 + rhs.y1,
            x2: self.x2 + rhs.x2,
            y2: self.y2 + rhs.y2,
        }
    }
}

/// Oriented rectangle defined by `f64` coordinates.
#[derive(Clone, Copy)]
pub struct DRect {
    /// Abscissa of first corner.
    pub x1: f64,
    /// Ordinate of first corner.
    pub y1: f64,
    /// Abscissa of second corner.
    pub x2: f64,
    /// Ordinate of second corner.
    pub y2: f64,
}

impl DRect {
    pub const ZERO: DRect = DRect::new(0.0, 0.0, 0.0, 0.0);

    /// Constructs a new rectangle.
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Constructs a new rectangle of null area at given position.
    pub const fn from_pos(pos: DVec2) -> Self {
        Self::new(pos.x, pos.y, pos.x, pos.y)
    }

    /// Constructs a new rectangle from a given position and size.
    /// `pos` is the position of the first corner, and second corner is at `pos + size`.
    pub const fn from_pos_size(pos: DVec2, size: DVec2) -> Self {
        Self::new(pos.x, pos.y, pos.x + size.x, pos.y + size.y)
    }

    /// Constructs a new rectangle from a given center and size.
    pub const fn from_center_size(pos: DVec2, size: DVec2) -> Self {
        Self::new(
            pos.x - size.x / 2.0,
            pos.y - size.y / 2.0,
            pos.x + size.x / 2.0,
            pos.y + size.y / 2.0,
        )
    }

    /// Constructs a rectangle from two corner coordinates.
    pub fn from_corners(x1y1: DVec2, x2y2: DVec2) -> Self {
        Self {
            x1: x1y1.x,
            y1: x1y1.y,
            x2: x2y2.x,
            y2: x2y2.y,
        }
    }

    /// Returns center of the rectangle.
    pub fn center(&self) -> DVec2 {
        DVec2 {
            x: self.x1.midpoint(self.x2),
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns left anchor of the rectangle.
    pub fn left(&self) -> DVec2 {
        DVec2 {
            x: self.x1,
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns right anchor of the rectangle.
    pub fn right(&self) -> DVec2 {
        DVec2 {
            x: self.x2,
            y: self.y1.midpoint(self.y2),
        }
    }

    /// Returns top anchor of the rectangle.
    pub fn top(&self) -> DVec2 {
        dvec2(self.x1.midpoint(self.x2), self.y1)
    }

    /// Returns bottom anchor of the rectangle.
    pub fn bottom(&self) -> DVec2 {
        dvec2(self.x1.midpoint(self.x2), self.y2)
    }

    /// Returns horizontal center of the rectangle.
    pub fn h_center(&self) -> f64 {
        self.x1.midpoint(self.x2)
    }

    /// Returns vertical center of the rectangle.
    pub fn v_center(&self) -> f64 {
        self.y1.midpoint(self.y2)
    }

    /// Returns a point in the rectangle using linear interpolation of the edge coordinates.
    ///
    /// For instance, `lerp_xy(0, 0.5)` will return the center of the left edge.
    pub fn lerp_xy(&self, x: f64, y: f64) -> DVec2 {
        dvec2(self.x1.lerp(self.x2, x), self.y1.lerp(self.y2, y))
    }

    /// Returns (x1, y1) corner coordinates as a vector.
    pub fn x1y1(&self) -> DVec2 {
        dvec2(self.x1, self.y1)
    }

    /// Returns (x1, y2) corner coordinates as a vector.
    pub fn x1y2(&self) -> DVec2 {
        dvec2(self.x1, self.y2)
    }

    /// Returns (x2, y1) corner coordinates as a vector.
    pub fn x2y1(&self) -> DVec2 {
        dvec2(self.x2, self.y1)
    }

    /// Returns (x2, y2) corner coordinates as a vector.
    pub fn x2y2(&self) -> DVec2 {
        dvec2(self.x2, self.y2)
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> f64 {
        self.x2 - self.x1
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> f64 {
        self.y2 - self.y1
    }

    /// Returns the area of the rectangle.
    pub fn area(&self) -> f64 {
        ((self.x2 - self.x1) * (self.y2 - self.y1)).abs()
    }

    /// Returns the dimensions of the rectangle. Items can be negative.
    pub fn size(&self) -> DVec2 {
        dvec2(self.x2 - self.x1, self.y2 - self.y1)
    }

    /// Returns either width or height depending on selected `axis`.
    pub fn size_axis(&self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.width(),
            Axis::Y => self.height(),
        }
    }

    /// Returns the minimum rectangle containing `self` and `other` rectangles.
    pub fn union(self, other: Self) -> Self {
        Self {
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
            x2: self.x2.max(other.x2),
            y2: self.y2.max(other.y2),
        }
    }

    /// Returns the maximum rectangle contained by `self` and `other` rectangles.
    pub fn intersection(self, other: Self) -> Self {
        Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        }
    }

    /// Returns true if `self` and `other` rectangles have non-null intersection.
    /// Used for AABB collision testing for instance.
    pub fn intersects(&self, other: &Self) -> bool {
        let overlap_x = self.x1 < other.x2 && self.x2 > other.x1;
        let overlap_y = self.y1 < other.y2 && self.y2 > other.y1;
        overlap_x && overlap_y
    }

    /// Returns true if given point is inside the rectangle, bounds inclusive.
    pub fn contains(&self, point: DVec2) -> bool {
        (self.x1..=self.x2).contains(&point.x) && (self.y1..=self.y2).contains(&point.y)
    }

    /// Returns true if given abscissa is between the rectangle left and right bounds inclusive.
    pub fn contains_x(&self, x: f64) -> bool {
        (self.x1..=self.x2).contains(&x)
    }

    /// Returns true if given ordinate is between the rectangle top and bottom bounds inclusive.
    pub fn contains_y(&self, x: f64) -> bool {
        (self.y1..=self.y2).contains(&x)
    }

    /// Expands the rectangle by `amount` on each side. If `amount` is negative, the rectangle
    /// shrinks. Note that the result can have negative width.
    pub fn expand(&self, amount: f64) -> Self {
        Self {
            x1: self.x1 - amount,
            y1: self.y1 - amount,
            x2: self.x2 + amount,
            y2: self.y2 + amount,
        }
    }

    /// Returns the rectangle with rounded coordinates.
    pub fn round(self) -> Self {
        Self {
            x1: self.x1.round(),
            y1: self.y1.round(),
            x2: self.x2.round(),
            y2: self.y2.round(),
        }
    }

    /// Creates a quad from the rectangle.
    pub fn to_quad(self) -> DQuad {
        self.into()
    }

    /// Returns the same rectangle but with `x1 <= x2` and `y1 < y2`.
    pub fn sorted(&self) -> Self {
        Self {
            x1: self.x1.min(self.x2),
            x2: self.x1.max(self.x2),
            y1: self.y1.min(self.y2),
            y2: self.y1.max(self.y2),
        }
    }
}

impl From<DVec2> for DRect {
    fn from(value: DVec2) -> Self {
        Self::from_pos(value)
    }
}

impl Add<DVec2> for DRect {
    type Output = DRect;

    fn add(self, rhs: DVec2) -> Self::Output {
        DRect {
            x1: self.x1 + rhs.x,
            y1: self.y1 + rhs.y,
            x2: self.x2 + rhs.x,
            y2: self.y2 + rhs.y,
        }
    }
}

impl Add<DRect> for DRect {
    type Output = DRect;

    fn add(self, rhs: DRect) -> Self::Output {
        Self {
            x1: self.x1 + rhs.x1,
            y1: self.y1 + rhs.y1,
            x2: self.x2 + rhs.x2,
            y2: self.y2 + rhs.y2,
        }
    }
}

/// Rectangle defined by `i32` coordinates.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
pub struct IRect {
    /// Abscissa of first corner.
    pub x1: i32,
    /// Ordinate of first corner.
    pub y1: i32,
    /// Abscissa of second corner.
    pub x2: i32,
    /// Ordinate of second corner.
    pub y2: i32,
}

impl IRect {
    pub const ZERO: Self = Self::new(0, 0, 0, 0);

    /// Constructs a new rectangle.
    pub const fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Returns the width of the rectangle `x2 - x1`.
    pub const fn width(&self) -> i32 {
        self.x2 - self.x1
    }

    /// Returns the height of the rectangle `y2 - y1`.
    pub const fn height(&self) -> i32 {
        self.y2 - self.y1
    }

    /// Returns the maximum rectangle contained by `self` and `other` rectangles.
    pub fn intersection(self, other: Self) -> Self {
        Self {
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
            x2: self.x2.min(other.x2),
            y2: self.y2.min(other.y2),
        }
    }
}
