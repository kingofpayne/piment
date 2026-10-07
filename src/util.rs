use image::Rgba;
use wgpu::TextureFormat;

/// Utility trait to get the corresponding texture format to an image.
pub trait ImageTextureFormat {
    fn texture_format() -> TextureFormat;
}

impl ImageTextureFormat for Rgba<u8> {
    fn texture_format() -> TextureFormat {
        TextureFormat::Rgba8Unorm
    }
}
