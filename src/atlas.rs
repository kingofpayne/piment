use image::{ImageBuffer, Pixel, Rgba, imageops};
use std::{cmp::Reverse, collections::BTreeMap};

/// Space kept between the atlas borders and around each packed glyph, so bilinear sampling of a
/// glyph cannot pull in a neighbour.
const MARGIN: u32 = 1;

/// Packs glyphs into a single image.
///
/// `K` is the glyphs keys types.
/// `D` is glyphs additional associated data type.
/// `P` is the atlas image pixel type.
pub struct Atlas<K: Ord, D, P: Pixel<Subpixel = u8> = Rgba<u8>> {
    glyphs: BTreeMap<K, Glyph<D, P>>,
    image: ImageBuffer<P, Vec<u8>>,
}

impl<K: Ord, D, P: Pixel<Subpixel = u8>> Atlas<K, D, P> {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            glyphs: BTreeMap::new(),
            image: ImageBuffer::new(width, height),
        }
    }

    /// Returns the packed glyph for `key`, or `None` if it is not in the atlas.
    pub(crate) fn glyph(&self, key: &K) -> Option<&Glyph<D, P>> {
        self.glyphs.get(key)
    }

    /// Returns every glyph of the atlas, with only the glyph data being modifiable.
    pub(crate) fn glyphs_mut(&mut self) -> impl Iterator<Item = GlyphRef<'_, D>> {
        self.glyphs.values_mut().map(|glyph| GlyphRef {
            data: &mut glyph.data,
            x: &glyph.x,
            y: &glyph.y,
            w: &glyph.w,
            h: &glyph.h,
        })
    }

    /// Returns the atlas image.
    pub(crate) fn image(&self) -> &ImageBuffer<P, Vec<u8>> {
        &self.image
    }

    /// Returns current image width.
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    /// Returns current image height.
    pub fn height(&self) -> u32 {
        self.image().height()
    }

    /// Insert or updates a glyph. This does not re-pack and re-build the atlas image, so
    /// [Self::rebuild] must be called once all glyphs have been inserted.
    pub fn insert(&mut self, key: K, image: ImageBuffer<P, Vec<u8>>, data: D) {
        self.glyphs.insert(
            key,
            Glyph {
                data,
                x: 0,
                y: 0,
                w: 0,
                h: 0,
                image,
            },
        );
    }

    /// Packs every glyph into the atlas image and stores where each one landed.
    ///
    /// Glyphs are shelf packed: tallest first, filling rows left to right, a new row being opened
    /// below the previous one once the right border is reached.
    ///
    /// Returns `Err` if the glyphs do not all fit in the atlas image.
    pub fn rebuild(&mut self) {
        // Clear previous image.
        self.image.fill(0);

        // Tallest glyphs first, so a shelf is only as high as the first glyph put on it.
        let mut glyphs: Vec<&mut Glyph<D, P>> = self.glyphs.values_mut().collect();
        glyphs.sort_by_key(|glyph| Reverse(glyph.image.height()));

        // Insertion point, plus the height of the shelf being filled.
        let mut x = MARGIN;
        let mut y = MARGIN;
        let mut shelf_h = 0;

        for glyph in glyphs {
            let w = glyph.image.width();
            let h = glyph.image.height();

            // Glyphs such as the space character have no picture and take no room.
            if w == 0 || h == 0 {
                glyph.x = 0;
                glyph.y = 0;
                glyph.w = 0;
                glyph.h = 0;
                continue;
            }

            // Open the next shelf when the glyph overflows the right border.
            if x + w + MARGIN > self.image.width() {
                x = MARGIN;
                y += shelf_h + MARGIN;
                shelf_h = 0;
            }
            // A glyph wider than the whole atlas overflows even a fresh shelf.
            if x + w + MARGIN > self.image.width() || y + h + MARGIN > self.image.height() {
                panic!();
            }

            imageops::replace(&mut self.image, &glyph.image, x as i64, y as i64);
            glyph.x = x as i32;
            glyph.y = y as i32;
            glyph.w = w as i32;
            glyph.h = h as i32;

            x += w + MARGIN;
            shelf_h = shelf_h.max(h);
        }
    }
}

/// Holds each atlas glyph data.
pub(crate) struct Glyph<D, P: Pixel<Subpixel = u8>> {
    /// Extra glyph data unrelated to glyph packing in the atlas.
    /// For font characters, this will store the character metrics.
    pub(crate) data: D,
    /// X offset in the atlas image.
    pub(crate) x: i32,
    /// Y offset in the atlas image.
    pub(crate) y: i32,
    /// Width in the atlas image.
    pub(crate) w: i32,
    /// Height in the atlas image.
    pub(crate) h: i32,
    /// Picture.
    /// Kepts aside the whole image so we can dynamically rebuild the atlas when new glyphs are
    /// added.
    image: ImageBuffer<P, Vec<u8>>,
}

/// Atlas glyph view where only the extra data can be modified.
pub(crate) struct GlyphRef<'a, D> {
    /// Extra glyph data unrelated to glyph packing in the atlas.
    pub(crate) data: &'a mut D,
    /// X offset in the atlas image.
    pub(crate) x: &'a i32,
    /// Y offset in the atlas image.
    pub(crate) y: &'a i32,
    /// Width in the atlas image.
    pub(crate) w: &'a i32,
    /// Height in the atlas image.
    pub(crate) h: &'a i32,
}
