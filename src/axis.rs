use glam::{DVec2, Vec2};

/// Used to select X or Y axis.
/// The numbering matches the order of components in a vector.
#[derive(Copy, Clone, Default, Eq, PartialEq)]
pub enum Axis {
    /// X axis.
    #[default]
    X = 0,
    /// Y axis.
    Y = 1,
}

impl Axis {
    pub fn as_dvec2(self) -> DVec2 {
        match self {
            Self::X => DVec2::new(1.0, 0.0),
            Self::Y => DVec2::new(0.0, 1.0),
        }
    }

    /// Returns the other axis.
    pub fn next(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }
}

pub trait AxisVec2<T> {
    /// Returns X or Y value depending on selected `axis`.
    fn axis(&self, axis: Axis) -> T;
}

impl AxisVec2<f32> for Vec2 {
    fn axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}

impl AxisVec2<f64> for DVec2 {
    fn axis(&self, axis: Axis) -> f64 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
        }
    }
}
