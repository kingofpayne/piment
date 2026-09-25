use crate::image_cache::ImageCache;
use image::{DynamicImage, GenericImageView};
use std::collections::BTreeMap;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

pub struct TextureManager {
    /// GPU device.
    device: Device,
    /// GPU queue.
    queue: Queue,
    /// Cache of the texture loaded in the GPU.
    textures: BTreeMap<String, Result<Texture, ()>>,
    /// Cache of the images loaded in RAM memory (not GPU).
    images: ImageCache,
}

impl TextureManager {
    pub fn new(device: Device, queue: Queue, images: ImageCache) -> Self {
        Self {
            device,
            queue,
            textures: BTreeMap::new(),
            images,
        }
    }

    /// Returns the texture corresponding to the given `source` path. If this is the first time the
    /// texture is queried, it is loaded from file, converted to the given texture format and
    /// transferred to the GPU memory.
    ///
    /// Possible values for `format` are `TextureFormat::Rgba8Unorm` and `TextureFormat::R8Unorm`.
    /// The later one is particularly usefull for loading microscopy images which are only in gray
    /// levels, making GPU video memory usage reduced (1 color channel instead of 3 or 4).
    ///
    /// Note: if the texture was already loaded previously, `format` is ignored; the texture is not
    /// loaded again in a different format.
    pub fn get(&mut self, source: &str, format: TextureFormat) -> Result<Texture, ()> {
        self.textures
            .entry(source.into())
            .or_insert_with(|| {
                let image = self.images.get_dynamic(source);
                create_texture_from_image(&self.device, &self.queue, &image, format)
            })
            .clone()
    }

    /// Non-blocking variant of [`TextureManager::get`].
    ///
    /// If the texture is already loaded it is returned immediately. Otherwise the underlying
    /// image is requested from the [`ImageCache`] without blocking: if it is not available yet
    /// the load is queued (to be handled by the background loader) and `None` is returned, so
    /// the caller can simply skip rendering until the texture becomes available on a later frame.
    pub fn need(&mut self, source: &str, format: TextureFormat) -> Option<Texture> {
        if let Some(result) = self.textures.get(source) {
            return result.clone().ok();
        }
        let image = self.images.need_dynamic(source)?;
        let result = create_texture_from_image(&self.device, &self.queue, &image, format);
        let texture = result.clone().ok();
        self.textures.insert(source.into(), result);
        texture
    }
}

/// Creates a [Texture] from a [DynamicImage].
///
/// `format` can be `TextureFormat::Rgba8Unorm` for color images with alpha channel, or
/// `TextureFormat::R8Unorm` for gray images.
///
/// Loading images in gray is interesting to reduce GPU video memory usage, especially when dealing
/// with only gray microscopy images.
pub fn create_texture_from_image(
    device: &Device,
    queue: &Queue,
    image: &DynamicImage,
    format: TextureFormat,
) -> Result<Texture, ()> {
    let dimensions = image.dimensions();
    // Depending on the selected texture format, we must convert the original image to a matching
    // pixel data buffer.
    let buf: &[u8] = match format {
        TextureFormat::Rgba8Unorm => &image.to_rgba8(),
        TextureFormat::R8Unorm => &image.to_luma8(),
        _ => panic!("Unsupported texture format"),
    };
    let texture_size = Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&TextureDescriptor {
        label: None,
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let pixel_size = match format {
        TextureFormat::Rgba8Unorm => 4,
        TextureFormat::R8Unorm => 1,
        _ => panic!(),
    };
    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        buf,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(pixel_size * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        texture_size,
    );
    Ok(texture)
}
