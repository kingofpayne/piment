use crate::color::Color;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec4};

/// Vertex structure used by the Painter for rendering 2D graphics.
///
/// Builder pattern is used to construct a [`Vertex`] depending on the required attributes like
/// color or texture coordinates:
///
/// ```
/// let v = Vertex::from_xy(1.0, 2.0).uv(0.0, 1.0).color(1.0, 0.0, 0.0, 1.0);
/// ```
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Vertex {
    pub xyz: [f32; 3],
    pub uv: [f32; 2],
    pub rgba1: [f32; 4],
    pub rgba2: [f32; 4],
    pub geom1: [f32; 4],
    pub geom2: [f32; 4],
    pub geom3: [f32; 4],
}

impl Vertex {
    /// Creates a new [`Vertex`] with given X and Y coordinates. Z coordinate is set to 0.0.
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self {
            xyz: [x, y, 0.0],
            uv: [0.0, 0.0],
            rgba1: [0.0, 0.0, 0.0, 1.0],
            rgba2: [0.0, 0.0, 0.0, 1.0],
            geom1: [0.0; 4],
            geom2: [0.0; 4],
            geom3: [0.0; 4],
        }
    }

    /// Creates a new [`Vertex`] with given X, Y and Z coordinates.
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            xyz: [x, y, z],
            uv: [0.0, 0.0],
            rgba1: [0.0, 0.0, 0.0, 1.0],
            rgba2: [0.0, 0.0, 0.0, 1.0],
            geom1: [0.0; 4],
            geom2: [0.0; 4],
            geom3: [0.0; 4],
        }
    }

    /// Creates a new [`Vertex`] from a given [`glam::Vec2`]. Z coordinate is set to 0.0.
    pub fn from_vec(xy: Vec2) -> Self {
        Self {
            xyz: [xy.x, xy.y, 0.0],
            uv: [0.0, 0.0],
            rgba1: [0.0, 0.0, 0.0, 1.0],
            rgba2: [0.0, 0.0, 0.0, 1.0],
            geom1: [0.0; 4],
            geom2: [0.0; 4],
            geom3: [0.0; 4],
        }
    }

    /// Builds Z coordinate.
    pub fn z(mut self, z: f32) -> Self {
        self.xyz[2] = z;
        self
    }

    /// Builds U and V texture coordinates of a vector.
    pub fn uv(mut self, u: f32, v: f32) -> Self {
        self.uv = [u, v];
        self
    }

    /// Builds RGBA color of a vector.
    pub fn rgba1(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.rgba1 = [r, g, b, a];
        self
    }

    /// Set same red, green and blue color components to same level `g`. Does not modify alpha
    /// component.
    pub fn gray(mut self, g: f32) -> Self {
        self.rgba1 = [g, g, g, self.rgba1[3]];
        self
    }

    /// Sets primary color of the vertex.
    pub fn color1(mut self, color: Color) -> Self {
        self.rgba1 = color.0;
        self
    }

    /// Sets secondary color of the vertex.
    pub fn color2(mut self, color: Color) -> Self {
        self.rgba2 = color.0;
        self
    }

    /// Sets shape geometry data 1.
    pub fn geom1(mut self, g1: f32, g2: f32, g3: f32, g4: f32) -> Self {
        self.geom1 = [g1, g2, g3, g4];
        self
    }

    /// Sets shape geometry data 1 from a vector.
    pub fn geom1_vec(mut self, value: Vec4) -> Self {
        self.geom1 = value.to_array();
        self
    }

    /// Sets shape geometry data 2.
    pub fn geom2(mut self, g1: f32, g2: f32, g3: f32, g4: f32) -> Self {
        self.geom2 = [g1, g2, g3, g4];
        self
    }

    /// Sets shape geometry data 2 from a vector.
    pub fn geom2_vec(mut self, value: Vec4) -> Self {
        self.geom2 = value.to_array();
        self
    }

    /// Sets shape geometry data 3.
    pub fn geom3(mut self, g1: f32, g2: f32, g3: f32, g4: f32) -> Self {
        self.geom3 = [g1, g2, g3, g4];
        self
    }

    /// Sets shape geometry data 3 from a vector.
    pub fn geom3_vec(mut self, value: Vec4) -> Self {
        self.geom3 = value.to_array();
        self
    }
}
