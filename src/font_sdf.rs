use crate::{
    font::{Atlas, CHARACTERS, TextHorizontalAlign, TextLayout, TextLayoutGlyph, system_font_data},
    rect::Rect,
    texture_manager::create_texture_from_image,
};
use glam::{Vec4, vec4};
use image::{DynamicImage, GrayImage, Luma};
use swash::{
    FontRef,
    scale::{Render, ScaleContext, Source, image::Image},
    zeno::Format,
};
use wgpu::{Device, Queue, Texture, TextureFormat};

/// Font size the glyphs distance fields are built for, in pixels. Texts of any size are rendered
/// by scaling these distance fields.
pub const CANONICAL_SIZE: f32 = 48.0;

/// Largest distance to a glyph contour stored in the atlas, in pixels at [CANONICAL_SIZE]. Farther
/// distances are clamped.
///
/// Every glyph tile is padded by that distance, so effects reaching outside of the contour, such as
/// outlines and shadows, must stay within it.
pub const SPREAD: u32 = 12;

/// Glyphs are rasterized this many times larger than [CANONICAL_SIZE], and their distance field is
/// downsampled afterwards, so the contour is located with sub-texel accuracy.
const SUPERSAMPLING: u32 = 4;

/// Width of the outline stroke, in pixels. The stroke is centered on the character contour, so it
/// bleeds half of that width outside of the plain character.
pub const OUTLINE_WIDTH: f32 = 2.0;

/// Shadow blur standard deviation per unit of font size: `sigma = size * SHADOW_SIGMA_PER_SIZE`.
pub const SHADOW_SIGMA_PER_SIZE: f32 = 1.0 / 11.0;

/// Atlas image width and height, in pixels.
const ATLAS_SIZE: u32 = 1024;

/// Squared distance given to pixels which have no seed to measure the distance to. Kept finite so
/// the distance transform arithmetic never produces NaN.
const FAR: f32 = 1e20;

/// A font rendered from Signed Distance Fields, which can be drawn at any size from a single atlas.
///
/// Each glyph of the atlas stores, for every texel, the distance to the glyph contour, positive
/// inside and negative outside. Distances are remapped from `[-SPREAD, SPREAD]` to `[0, 1]`, so
/// the contour lies at 0.5. The GPU interpolates these distances when sampling the atlas, which
/// keeps the contour smooth when magnified, and outlines and shadows are obtained by moving and
/// softening the contour threshold.
///
/// Small texts look sharper with [crate::font::Font], which rasterizes hinted glyphs at their exact
/// size.
pub struct FontSdf {
    /// Glyphs atlas.
    atlas: Atlas<char, FontSdfGlyph, Luma<u8>>,
    /// Tallest character ink above the baseline, in pixels at [CANONICAL_SIZE].
    ascent: f32,
    /// Deepest character ink below the baseline, in pixels at [CANONICAL_SIZE].
    descent: f32,
    /// WGPU texture.
    texture: Option<Texture>,
}

impl FontSdf {
    /// Resolves a system sans-serif and builds the atlas from it.
    pub fn from_system() -> Self {
        let (data, index) = system_font_data();
        Self::from_data(&data, index)
    }

    /// Builds the atlas from font `data`, computing the distance field of every character of
    /// [CHARACTERS].
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self::from_data(&data, 0)
    }

    fn from_data(data: &[u8], index: u32) -> Self {
        let font = FontRef::from_index(data, index as usize).expect("Failed to parse font file");
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(&[]).scale(CANONICAL_SIZE);
        let mut context = ScaleContext::new();
        // Hinting snaps the outlines to the pixel grid of one size, which is wrong once scaled.
        let mut scaler = context
            .builder(font)
            .size(CANONICAL_SIZE * SUPERSAMPLING as f32)
            .hint(false)
            .build();
        let mut atlas = Atlas::new(ATLAS_SIZE, ATLAS_SIZE);
        let mut ascent = 0.0f32;
        let mut descent = 0.0f32;

        for char in CHARACTERS.chars() {
            let id = charmap.map(char);
            let mask = Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .render(&mut scaler, id);
            let (image, x, y) = match mask {
                Some(mask) if mask.placement.width > 0 && mask.placement.height > 0 => {
                    let s = SUPERSAMPLING as f32;
                    let top = mask.placement.top;
                    ascent = ascent.max(top as f32 / s);
                    descent = descent.max((mask.placement.height as i32 - top) as f32 / s);
                    distance_field(&mask)
                }
                // Characters such as the space have nothing to render at all.
                _ => (GrayImage::new(0, 0), 0, 0),
            };
            atlas.insert(
                char,
                image,
                FontSdfGlyph {
                    x: x as f32,
                    y: y as f32,
                    advance: metrics.advance_width(id),
                    uv: Vec4::ZERO,
                },
            );
        }

        atlas.rebuild();

        // Packing is only known once the atlas is built, so texture coordinates are calculated
        // afterwards.
        let width = atlas.image().width() as f32;
        let height = atlas.image().height() as f32;
        for glyph in atlas.glyphs_mut() {
            let u1 = glyph.x as f32 / width;
            let v1 = glyph.y as f32 / height;
            let u2 = u1 + glyph.w as f32 / width;
            let v2 = v1 + glyph.h as f32 / height;
            glyph.data.uv = vec4(u1, v1, u2, v2);
        }

        Self {
            atlas,
            ascent,
            descent,
            texture: None,
        }
    }

    /// Builds the WGPU texture from the atlas image.
    pub fn build_texture(&mut self, device: &Device, queue: &Queue) {
        let image = DynamicImage::ImageLuma8(self.atlas.image().clone());
        let texture = create_texture_from_image(device, queue, &image, TextureFormat::R8Unorm)
            .expect("Failed to create the font atlas texture");
        self.texture = Some(texture);
    }

    /// Returns the atlas texture, or `None` if [Self::build_texture] has not been called yet.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    /// Returns the tallest character ink above the baseline for the given font `size`.
    pub fn ascent(&self, size: f32) -> f32 {
        self.ascent * size / CANONICAL_SIZE
    }

    /// Returns the deepest character ink below the baseline for the given font `size`.
    pub fn descent(&self, size: f32) -> f32 {
        self.descent * size / CANONICAL_SIZE
    }

    /// Calculate the width of a string for the given font `size`.
    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        let width: f32 = text
            .chars()
            .filter_map(|char| self.atlas.glyph(&char))
            .map(|glyph| glyph.data.advance)
            .sum();
        width * size / CANONICAL_SIZE
    }

    /// Builds a [TextLayout] positioning each character of `text` in `rect`, for the given font
    /// `size`.
    pub fn layout(
        &self,
        text: &str,
        rect: Rect,
        align: TextHorizontalAlign,
        size: f32,
    ) -> TextLayout {
        let scale = size / CANONICAL_SIZE;
        let mut glyphs = Vec::new();
        // Only the text origin is rounded: glyphs contours are drawn at sub-pixel positions without
        // getting blurry.
        let mut x = match align {
            TextHorizontalAlign::Left => rect.x1,
            TextHorizontalAlign::Right => rect.x2 - self.text_width(text, size),
            TextHorizontalAlign::Center => rect.h_center() - self.text_width(text, size) / 2.0,
        }
        .round();
        let baseline = (rect.v_center() + (self.ascent(size) - self.descent(size)) / 2.0).round();
        let mut bounds = Rect::new(x, rect.y1, x, rect.y1);
        for char in text.chars() {
            let Some(glyph) = self.atlas.glyph(&char) else {
                continue;
            };
            if glyph.w > 0 {
                let x1 = x + glyph.data.x * scale;
                let x2 = x1 + glyph.w as f32 * scale;
                let y1 = baseline + glyph.data.y * scale;
                let y2 = y1 + glyph.h as f32 * scale;

                glyphs.push(TextLayoutGlyph {
                    xy: Rect::new(x1, y1, x2, y2),
                    uv: Rect::new(
                        glyph.data.uv.x,
                        glyph.data.uv.y,
                        glyph.data.uv.z,
                        glyph.data.uv.w,
                    ),
                });
                bounds.x1 = bounds.x1.min(x1);
                bounds.x2 = bounds.x2.max(x2);
                bounds.y1 = bounds.y1.min(y1);
                bounds.y2 = bounds.y2.max(y2);
            }
            x += glyph.data.advance * scale;
        }
        TextLayout { glyphs, bounds }
    }
}

/// Builds the distance field tile of a glyph from its supersampled alpha `mask`.
///
/// The tile covers the mask padded by [SPREAD] on every side. Returns the tile picture, and the
/// offset of its top left corner from the character origin on the text baseline, in pixels at
/// [CANONICAL_SIZE], Y pointing down.
fn distance_field(mask: &Image) -> (GrayImage, i32, i32) {
    let s = SUPERSAMPLING as i32;
    let spread = SPREAD as i32;
    let placement = &mask.placement;

    // Mask bounds in supersampled pixels. `placement.top` is the distance from the origin up to
    // the top of the mask, hence the sign flip to get Y pointing down.
    let mx1 = placement.left;
    let my1 = -placement.top;
    let mx2 = mx1 + placement.width as i32;
    let my2 = my1 + placement.height as i32;

    // Tile bounds, aligned on the canonical pixel grid so every tile texel covers exactly
    // `s * s` supersampled pixels.
    let x1 = mx1.div_euclid(s) - spread;
    let y1 = my1.div_euclid(s) - spread;
    let x2 = (mx2 + s - 1).div_euclid(s) + spread;
    let y2 = (my2 + s - 1).div_euclid(s) + spread;
    let w = (x2 - x1) as usize;
    let h = (y2 - y1) as usize;
    let sw = w * s as usize;
    let sh = h * s as usize;

    let mut inside = vec![false; sw * sh];
    for my in 0..placement.height as usize {
        for mx in 0..placement.width as usize {
            let value = mask.data[mx + my * placement.width as usize];
            let x = (mx1 - x1 * s) as usize + mx;
            let y = (my1 - y1 * s) as usize + my;
            inside[x + y * sw] = value >= 128;
        }
    }
    let to_inside = squared_distances(&inside, sw, sh, true);
    let to_outside = squared_distances(&inside, sw, sh, false);

    let s = s as usize;
    let mut image = GrayImage::new(w as u32, h as u32);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let mut sum = 0.0;
        for sy in 0..s {
            for sx in 0..s {
                let i = (x as usize * s + sx) + (y as usize * s + sy) * sw;
                // Distances are measured between pixel centers, so the contour lies half a pixel
                // before the nearest pixel of the other side.
                sum += if inside[i] {
                    to_outside[i].sqrt() - 0.5
                } else {
                    0.5 - to_inside[i].sqrt()
                };
            }
        }
        let distance = sum / (s * s * s) as f32;
        let value = 0.5 + distance / (2.0 * SPREAD as f32);
        pixel.0[0] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    }

    (image, x1, y1)
}

/// Returns, for every pixel of a `w` by `h` grid, the squared Euclidean distance to the nearest
/// pixel whose `seeds` value equals `seed`.
///
/// This is the separable transform from Felzenszwalb and Huttenlocher: one dimensional transforms
/// on every column, then on every row of the result.
fn squared_distances(seeds: &[bool], w: usize, h: usize, seed: bool) -> Vec<f32> {
    let mut grid: Vec<f32> = seeds
        .iter()
        .map(|&value| if value == seed { 0.0 } else { FAR })
        .collect();
    let n = w.max(h);
    let mut f = vec![0.0; n];
    let mut d = vec![0.0; n];
    let mut v = vec![0; n];
    let mut z = vec![0.0; n + 1];

    for x in 0..w {
        for y in 0..h {
            f[y] = grid[x + y * w];
        }
        squared_distances_1d(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            grid[x + y * w] = d[y];
        }
    }
    for row in grid.chunks_exact_mut(w) {
        f[..w].copy_from_slice(row);
        squared_distances_1d(&f[..w], row, &mut v, &mut z);
    }
    grid
}

/// One dimensional squared distance transform of `f` into `d`, computing the lower envelope of the
/// parabolas rooted at every sample.
///
/// `v` and `z` are working buffers, holding the parabolas of the envelope and the boundaries
/// between them. `v` must be at least as long as `f`, and `z` one element longer.
fn squared_distances_1d(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let mut k = 0;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..f.len() {
        let qf = q as f32;
        let mut s;
        // Terminates before `k` underflows since `z[0]` is below any finite intersection.
        loop {
            let p = v[k] as f32;
            s = ((f[q] + qf * qf) - (f[v[k]] + p * p)) / (2.0 * (qf - p));
            if s > z[k] {
                break;
            }
            k -= 1;
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f32::INFINITY;
    }
    k = 0;
    for (q, distance) in d.iter_mut().enumerate() {
        let qf = q as f32;
        while z[k + 1] < qf {
            k += 1;
        }
        let p = v[k] as f32;
        *distance = (qf - p) * (qf - p) + f[v[k]];
    }
}

/// Character metrics and atlas location.
struct FontSdfGlyph {
    /// Tile x offset from the pen position, in pixels at [CANONICAL_SIZE].
    x: f32,
    /// Tile y offset from the text baseline, Y pointing down, in pixels at [CANONICAL_SIZE].
    y: f32,
    /// Character position advance, in pixels at [CANONICAL_SIZE].
    advance: f32,
    /// Texture coordinates.
    uv: Vec4,
}
