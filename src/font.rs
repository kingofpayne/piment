use crate::{TextureAtlas, rect::Rect};
use font_kit::{
    family_name::FamilyName, handle::Handle, properties::Properties, source::SystemSource,
};
use glam::{Vec4, vec4};
use image::{GrayImage, RgbaImage, imageops};
use std::collections::BTreeMap;
use swash::{
    FontRef,
    scale::{Render, ScaleContext, Source, image::Image},
    zeno::{Cap, Format, Join, Stroke},
};
use wgpu::{Device, Queue, Texture};

/// Characters rasterized in the atlas.
pub(crate) const CHARACTERS: &str = concat!(
    "abcdefghijklmnopqrstuvwxyz",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    "0123456789",
    ".:;,/\\?$*+-=()[]{}#\"'|_@ !<>%µ&^",
);

/// Width of the outline stroke, in pixels. The stroke is centered on the character contour, so it
/// bleeds half of that width outside of the plain character.
const OUTLINE_WIDTH: f32 = 2.0;

/// Shadow Gaussian blur standard deviation per unit of font size: `sigma = scale * SHADOW_SIGMA_PER_SIZE`.
const SHADOW_SIGMA_PER_SIZE: f32 = 1.0 / 11.0;

/// Atlas image width and height, in pixels.
const ATLAS_SIZE: u32 = 1024;

pub struct Font {
    /// Glyphs atlas
    atlas: TextureAtlas<FontGlyphKey, FontGlyph>,
    /// Character set ink extents for each built font size.
    metrics: BTreeMap<i32, Metrics>,
}

/// How far the ink of a rasterized character set reaches on both sides of the baseline, in pixels.
///
/// These are measured from the rendered glyphs rather than taken from the typographic metrics,
/// because they are used to vertically center text in a rectangle.
struct Metrics {
    /// Tallest character ink above the baseline.
    ascent: f32,
    /// Deepest character ink below the baseline.
    descent: f32,
}

impl Font {
    /// Resolves a system sans-serif and builds the atlas at each of the requested `sizes`.
    ///
    /// Prefers DejaVu Sans when it is installed, then other common UI faces, then the platform
    /// generic sans-serif.
    pub fn from_system(sizes: &[i32]) -> Self {
        let (data, index) = system_font_data();
        Self::from_data(&data, index, sizes)
    }

    /// Builds the atlas from font `data`, rasterizing every character of [CHARACTERS] at each of
    /// the requested `sizes`.
    ///
    /// Each character is rendered three times into its atlas tile: filled in the red channel,
    /// stroked in the green channel, and filled then gaussian blurred in the blue channel, to be
    /// used as a shadow. All renderings share the tile, so a character, its outline and its
    /// shadow are painted with a single quad, picking a channel with the fragment shader.
    pub fn from_bytes(data: Vec<u8>, sizes: &[i32]) -> Self {
        Self::from_data(&data, 0, sizes)
    }

    fn from_data(data: &[u8], index: u32, sizes: &[i32]) -> Self {
        let font = FontRef::from_index(data, index as usize).expect("Failed to parse font file");
        let charmap = font.charmap();
        let mut context = ScaleContext::new();
        let mut atlas = TextureAtlas::new(ATLAS_SIZE, ATLAS_SIZE);
        let mut metrics_by_size = BTreeMap::new();

        // Round the stroke corners and ends, as the sharp spikes a miter join makes on the
        // tight angles of a character look like rendering glitches.
        let mut stroke = Stroke::new(OUTLINE_WIDTH);
        stroke.cap(Cap::Round).join(Join::Round);

        for &scale in sizes {
            let metrics = font.glyph_metrics(&[]).scale(scale as f32);
            let mut scaler = context.builder(font).size(scale as f32).hint(true).build();
            let sigma = SHADOW_SIGMA_PER_SIZE * scale as f32;
            let mut ascent = 0;
            let mut descent = 0;

            for char in CHARACTERS.chars() {
                let id = charmap.map(char);
                let plain = Render::new(&[Source::Outline])
                    .format(Format::Alpha)
                    .render(&mut scaler, id);
                let outline = Render::new(&[Source::Outline])
                    .format(Format::Alpha)
                    .style(stroke)
                    .render(&mut scaler, id);

                // The outline stroke bleeds outside of the character, so only the plain rendering
                // tells how far the characters really reach. `placement.top` is the distance from
                // the baseline up to the top of the mask, so what is left of its height falls
                // below the baseline.
                if let Some(image) = plain.as_ref() {
                    ascent = ascent.max(image.placement.top);
                    descent = descent.max(image.placement.height as i32 - image.placement.top);
                }

                let (image, x, y) = tile(plain.as_ref(), outline.as_ref(), sigma);
                atlas
                    .inner_mut()
                    .insert(
                        FontGlyphKey { scale, char },
                        image,
                        FontGlyph {
                            x,
                            y,
                            advance: metrics.advance_width(id).round() as i32,
                            uv: Vec4::ZERO,
                        },
                    )
                    .unwrap();
            }

            metrics_by_size.insert(
                scale,
                Metrics {
                    ascent: ascent as f32,
                    descent: descent as f32,
                },
            );
        }

        // Repacking changes glyph positions, so calculate texture coordinates afterwards.
        let width = atlas.inner().width() as f32;
        let height = atlas.inner().height() as f32;
        for glyph in atlas.glyphs_mut() {
            let u1 = *glyph.x as f32 / width;
            let v1 = *glyph.y as f32 / height;
            let u2 = u1 + *glyph.w as f32 / width;
            let v2 = v1 + *glyph.h as f32 / height;
            glyph.data.uv = vec4(u1, v1, u2, v2);
        }

        Self {
            atlas,
            metrics: metrics_by_size,
        }
    }

    pub fn update_texture(&mut self, device: &Device, queue: &Queue) {
        self.atlas.update_texture(device, queue);
    }

    /// Returns the atlas texture, or `None` if [Self::build_texture] has not been called yet.
    pub fn texture(&self) -> Option<&Texture> {
        self.atlas.texture()
    }

    /// Returns the tallest character ink above the baseline for the given font `size`.
    pub fn ascent(&self, size: i32) -> f32 {
        self.metrics.get(&size).map_or(0.0, |m| m.ascent)
    }

    /// Returns the deepest character ink below the baseline for the given font `size`.
    pub fn descent(&self, size: i32) -> f32 {
        self.metrics.get(&size).map_or(0.0, |m| m.descent)
    }

    /// Calculate the width of a string for the given font `size`.
    pub fn text_width(&self, text: &str, size: i32) -> f32 {
        let mut x = 0.0;
        for char in text.chars() {
            let Some(glyph) = self
                .atlas
                .inner()
                .glyph(&FontGlyphKey { scale: size, char })
            else {
                continue;
            };
            x += glyph.data.advance as f32;
        }
        x
    }

    /// Builds a [TextLayout] positioning each character of `text` in `rect`, for the given font
    /// `size`.
    pub fn layout(
        &self,
        text: &str,
        rect: Rect,
        align: TextHorizontalAlign,
        size: i32,
    ) -> TextLayout {
        let mut glyphs = Vec::new();
        // x and baseline will be rounded to avoid rendering blurry text.
        let mut x = match align {
            TextHorizontalAlign::Left => rect.x1,
            TextHorizontalAlign::Right => rect.x2 - self.text_width(text, size),
            TextHorizontalAlign::Center => rect.h_center() - self.text_width(text, size) / 2.0,
        }
        .round();
        // The character set ink spans from `ascent` above the baseline down to `descent` below it,
        // and that box is what gets centered on the rectangle. Both extents are constants for a
        // given size, so two texts sharing a rectangle stay aligned whatever glyphs they use.
        let baseline = (rect.v_center() + (self.ascent(size) - self.descent(size)) / 2.0).round();
        let mut bounds = Rect::new(x, rect.y1, x, rect.y1);
        for char in text.chars() {
            let Some(glyph) = self
                .atlas
                .inner()
                .glyph(&FontGlyphKey { scale: size, char })
            else {
                continue;
            };
            // Some characters, such as " ", have no image.
            if glyph.w > 0 {
                let x1 = x + glyph.data.x as f32;
                let x2 = x1 + glyph.w as f32;
                let y1 = baseline + glyph.data.y as f32;
                let y2 = y1 + glyph.h as f32;

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
            x += glyph.data.advance as f32;
        }
        TextLayout { glyphs, bounds }
    }
}

/// Resolves a system sans-serif font, and returns the font file data and the font index in that
/// file.
///
/// Prefers DejaVu Sans when it is installed, then other common UI faces, then the platform generic
/// sans-serif.
pub(crate) fn system_font_data() -> (Vec<u8>, u32) {
    let handle = SystemSource::new()
        .select_best_match(
            &[
                FamilyName::Title("DejaVu Sans".into()),
                FamilyName::Title("Liberation Sans".into()),
                FamilyName::Title("Noto Sans".into()),
                FamilyName::Title("Arial".into()),
                FamilyName::Title("Helvetica".into()),
                FamilyName::SansSerif,
            ],
            &Properties::new(),
        )
        .expect("Failed to find a system sans-serif font");
    match handle {
        Handle::Path { path, font_index } => (
            std::fs::read(&path).expect("Failed to load system font file"),
            font_index,
        ),
        Handle::Memory { bytes, font_index } => {
            (std::sync::Arc::unwrap_or_clone(bytes), font_index)
        }
    }
}

/// Merges the filled and stroked renderings of a character into a single RGBA tile, the fill going
/// to the red channel and the stroke to the green one. The blue channel gets the fill blurred with
/// a gaussian of standard deviation `sigma`, in pixels, to be used as a shadow.
///
/// Both renderings are placed relatively to the character origin and have different sizes, so the
/// tile covers the union of their bounding boxes, padded by three `sigma` on every side so the
/// blurred shadow is not cut off. Returns the tile picture, and the offset of its top left corner
/// from the character origin on the text baseline, Y pointing down.
fn tile(plain: Option<&Image>, outline: Option<&Image>, sigma: f32) -> (RgbaImage, i32, i32) {
    // Characters such as the space have nothing to render at all.
    let masks: Vec<&Image> = [plain, outline]
        .into_iter()
        .flatten()
        .filter(|mask| mask.placement.width > 0 && mask.placement.height > 0)
        .collect();
    if masks.is_empty() {
        return (RgbaImage::new(0, 0), 0, 0);
    }

    // `placement.top` is the distance from the origin up to the top of the mask, hence the sign
    // flip to get Y pointing down.
    let pad = (3.0 * sigma).ceil() as i32;
    let x1 = masks.iter().map(|m| m.placement.left).min().unwrap() - pad;
    let y1 = masks.iter().map(|m| -m.placement.top).min().unwrap() - pad;
    let x2 = masks
        .iter()
        .map(|m| m.placement.left + m.placement.width as i32)
        .max()
        .unwrap()
        + pad;
    let y2 = masks
        .iter()
        .map(|m| -m.placement.top + m.placement.height as i32)
        .max()
        .unwrap()
        + pad;

    let mut image =
        RgbaImage::from_pixel((x2 - x1) as u32, (y2 - y1) as u32, [0, 0, 0, 255].into());
    // Plain characters go to the red channel, outline strokes to the green one.
    for (mask, channel) in [(plain, 0), (outline, 1)] {
        let Some(mask) = mask else { continue };
        let w = mask.placement.width;
        let h = mask.placement.height;
        let x = mask.placement.left - x1;
        let y = -mask.placement.top - y1;
        for my in 0..h {
            for mx in 0..w {
                let value = mask.data[(mx + my * w) as usize];
                image.get_pixel_mut((x as u32) + mx, (y as u32) + my).0[channel] = value;
            }
        }
    }

    // The shadow is the plain character, blurred.
    let plain = GrayImage::from_fn(image.width(), image.height(), |x, y| {
        [image.get_pixel(x, y).0[0]].into()
    });
    let shadow = imageops::blur(&plain, sigma);
    for (pixel, value) in image.pixels_mut().zip(shadow.pixels()) {
        pixel.0[2] = value.0[0];
    }

    (image, x1, y1)
}

/// Identifies a font character for a given font scale in the atlas.
#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct FontGlyphKey {
    /// Font size.
    scale: i32,
    /// Character.
    char: char,
}

pub struct FontGlyph {
    /// Character x offset, from the pen position.
    x: i32,
    /// Character y offset, from the text baseline, Y pointing down.
    y: i32,
    /// Character position advance.
    advance: i32,
    /// Texture coordinates.
    /// Calculated from glyph coordinates in the atlas - this is redundant but useful for
    /// performance.
    uv: Vec4,
}

/// Holds position of glyphs for a text rendering.
pub struct TextLayout {
    pub glyphs: Vec<TextLayoutGlyph>,
    pub bounds: Rect,
}

pub struct TextLayoutGlyph {
    /// Quad coordinates.
    pub xy: Rect,
    /// Quad texture coordinates.
    pub uv: Rect,
}

/// Possible horizontal alignment for laying out text.
#[derive(Copy, Clone)]
pub enum TextHorizontalAlign {
    Left,
    Right,
    Center,
}
