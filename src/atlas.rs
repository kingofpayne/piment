use image::{ImageBuffer, Pixel, Rgba, imageops};
use std::{collections::BTreeMap, mem::take};
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyTextureInfo, Texture, TextureAspect,
    TextureDimension, TextureUsages,
};

use crate::util::ImageTextureFormat;

/// Space kept between the atlas borders and around each packed glyph, so bilinear sampling of a
/// glyph cannot pull in a neighbour.
const MARGIN: u32 = 1;

/// Packs glyphs into a single image.
///
/// `K` is the glyphs keys types.
/// `D` is glyphs additional associated data type.
/// `P` is the atlas image pixel type.
pub struct Atlas<K: Ord, D, P: Pixel<Subpixel = u8> = Rgba<u8>> {
    /// Stores glyphs position, data and image.
    glyphs: BTreeMap<K, Glyph<D, P>>,
    /// Assembled image, containing all the glyphs.
    image: ImageBuffer<P, Vec<u8>>,
    /// Abscissa of the insertion cursor.
    x: u32,
    /// Ordinate of the insertion cursor.
    y: u32,
    /// Current insertion row height.
    row_h: u32,
}

impl<K: Ord, D, P: Pixel<Subpixel = u8>> Atlas<K, D, P> {
    /// Creates a new atlas of the given image size.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            glyphs: BTreeMap::new(),
            image: ImageBuffer::new(width, height),
            x: MARGIN,
            y: MARGIN,
            row_h: 0,
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
        self.image.height()
    }

    /// Inserts or updates a glyph, packing it at the end of the current row.
    ///
    /// Inserting a new glyph does not invalidate already present glyphs, but packing may therefore
    /// waste space. [Self::repack] can be called to packs all glyphs more optimally.
    ///
    /// Updating an existing glyph does not reclaim its previous allocation.
    pub fn insert(
        &mut self,
        key: K,
        image: ImageBuffer<P, Vec<u8>>,
        data: D,
    ) -> Result<(), AtlasPackError> {
        let w = image.width();
        let h = image.height();

        // Glyphs such as the space character have no picture and take no room.
        if w == 0 || h == 0 {
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
            return Ok(());
        }

        let mut x = self.x;
        let mut y = self.y;
        let mut row_h = self.row_h;

        // Open the next row when the glyph overflows the right border.
        if x + w + MARGIN > self.image.width() {
            x = MARGIN;
            y += row_h + MARGIN;
            row_h = 0;
        }
        // A glyph wider than the whole atlas overflows even a fresh row.
        if x + w + MARGIN > self.image.width() || y + h + MARGIN > self.image.height() {
            return Err(AtlasPackError);
        }

        imageops::replace(&mut self.image, &image, x as i64, y as i64);
        self.glyphs.insert(
            key,
            Glyph {
                data,
                x: x as i32,
                y: y as i32,
                w: w as i32,
                h: h as i32,
                image,
            },
        );

        self.x = x + w + MARGIN;
        self.y = y;
        self.row_h = row_h.max(h);
        Ok(())
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
    /// Kept aside so the atlas can dynamically repack its glyphs.
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

/// This error is returned by [Atlas::insert] when packing a new glyph fails because it lacks room.
#[derive(Debug)]
pub struct AtlasPackError;

/// An atlas and its associated [Texture] that can be used for GPU rendering.
///
/// Manage texture update through a dirty flag which is set when the atlas is modified.
///
/// `K` is the glyphs keys types.
/// `D` is glyphs additional associated data type.
/// `P` is the atlas image pixel type.
pub struct TextureAtlas<K: Ord, D, P: Pixel<Subpixel = u8> = Rgba<u8>>
where
    P: ImageTextureFormat,
{
    inner: Atlas<K, D, P>,
    /// WGPU texture.
    texture: Option<Texture>,
    /// When set, the texture content must be updated from the atlas image.
    dirty: bool,
}

impl<K: Ord, D, P: Pixel<Subpixel = u8>> TextureAtlas<K, D, P>
where
    P: ImageTextureFormat,
{
    /// Creates a new atlas of the given image size.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: Atlas::new(width, height),
            texture: None,
            dirty: true,
        }
    }

    /// Returns reference to the inner atlas.
    pub fn inner(&self) -> &Atlas<K, D, P> {
        &self.inner
    }

    /// Returns mutable reference to the inner atlas.
    /// This sets the dirty flag, so this method shall not be called if no modification of the atlas
    /// is intended.
    pub fn inner_mut(&mut self) -> &mut Atlas<K, D, P> {
        self.dirty = true;
        &mut self.inner
    }

    /// Returns every glyph of the atlas, with only the glyph data being modifiable.
    /// As only the custom data can be modified, the atlas image is not impacted and therefore the
    /// dirty flag remains untouched.
    pub(crate) fn glyphs_mut(&mut self) -> impl Iterator<Item = GlyphRef<'_, D>> {
        self.inner.glyphs_mut()
    }

    /// Ensure the texture is up-to-date after glyphs atlas modification.
    ///
    /// This creates the texture if not already initialized, and upload to the GPU texture the image
    /// content if atlas was modified. Finally, clears the dirty flag.
    pub fn update_texture(&mut self, device: &Device, queue: &Queue) {
        // Create texture if it does not exist yet.
        let texture = self.texture.get_or_insert_with(|| {
            let size = Extent3d {
                width: self.inner.width(),
                height: self.inner.height(),
                depth_or_array_layers: 1,
            };
            device.create_texture(&wgpu::wgt::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: P::texture_format(),
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            })
        });
        // When dirty flag is set, clear it and upload image content to the GPU texture.
        if take(&mut self.dirty) {
            let size = Extent3d {
                width: self.inner.width(),
                height: self.inner.height(),
                depth_or_array_layers: 1,
            };
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &self.inner.image,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        P::texture_format().block_copy_size(None).unwrap() * self.inner.width(),
                    ),
                    rows_per_image: Some(self.inner.height()),
                },
                size,
            );
        }
    }

    /// Returns reference to the texture, or None if not yet initialized.
    pub fn texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }
}
