use crate::{
    event::CustomEvent, font::Font, image_cache::ImageCache, painter::Painter,
    texture_manager::TextureManager, theme::THEME,
};
use glam::uvec2;
use image::{GrayImage, ImageBuffer, Luma};
use pollster::FutureExt;
use std::sync::Arc;
use wgpu::{
    Backends, CompositeAlphaMode, Device, DeviceDescriptor, DeviceType, Extent3d,
    InstanceDescriptor, Origin3d, PresentMode, Queue, Surface, SurfaceConfiguration,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

pub struct Graphics {
    pub window: Arc<Window>,
    pub device: Device,
    pub queue: Queue,
    pub size: PhysicalSize<u32>,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub surface_format: TextureFormat,
    pub texture_depth: Texture,
    pub painter: Painter,
    pub textures: TextureManager,
    pub font: Font,
    /// Sends custom event to winit main event loop.
    /// Can be used by threads to wake-up and notify main loop.
    pub proxy: EventLoopProxy<CustomEvent>,
}

impl Graphics {
    pub fn new(
        window: Arc<Window>,
        proxy: EventLoopProxy<CustomEvent>,
        images: ImageCache,
    ) -> Self {
        let instance = wgpu::Instance::new(&InstanceDescriptor::default());

        let mut adapters: Vec<_> = instance.enumerate_adapters(Backends::PRIMARY);
        // There can be multiple adapters, and we prefer selecting a GPU adapter rather than a CPU
        // one. We sort the adapters and pick the best.
        adapters.sort_by_key(|x| match x.get_info().device_type {
            DeviceType::Other => 4,
            DeviceType::IntegratedGpu => 1,
            DeviceType::DiscreteGpu => 0,
            DeviceType::VirtualGpu => 3,
            DeviceType::Cpu => 2,
        });
        let adapter = adapters[0].clone();
        println!("Running on adapter: {:#?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor::default())
            .block_on()
            .unwrap();
        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0].remove_srgb_suffix();
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![surface_format],
        };
        surface.configure(&device, &surface_config);

        let mut font = Font::from_system(&[THEME.font_size]);
        font.build_texture(&device, &queue);

        let painter = Painter::new(
            &device,
            surface_config.view_formats[0],
            uvec2(size.width, size.height),
        );

        let texture_depth = create_depth_texture(&device, size);

        Self {
            window,
            device: device.clone(),
            queue: queue.clone(),
            size,
            surface,
            surface_config,
            surface_format,
            texture_depth,
            painter,
            textures: TextureManager::new(device, queue, images),
            font,
            proxy,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.texture_depth = create_depth_texture(&self.device, new_size);
        self.painter.size = uvec2(new_size.width, new_size.height);
    }

    /// Creates a basic 2D texture.
    pub fn create_texture(&self, width: u32, height: u32, format: TextureFormat) -> Texture {
        self.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Updates a GPU texture by queuing a CPU to GPU buffer copy.
    pub fn update_texture<P, C>(&self, texture: &Texture, image: &ImageBuffer<P, C>)
    where
        P: image::Pixel<Subpixel = u8>,
        C: std::ops::Deref<Target = [u8]>,
    {
        let texture_size = Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };
        self.queue.write_texture(
            TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            image,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(image.width() * size_of::<P>() as u32),
                rows_per_image: Some(image.height()),
            },
            texture_size,
        );
    }

    /// Creates a checkerboard texture. `size` must be a multiple of 2.
    pub fn create_checkerboard_texture(&self, size: u32, n: u32) -> Texture {
        debug_assert!(size.is_multiple_of(2));
        let texture = self.create_texture(size, size, TextureFormat::R8Unorm);
        let w = size / (2 * n);
        let image = GrayImage::from_fn(size, size, |x, y| {
            if ((x / w) % 2) ^ ((y / w) % 2) == 0 {
                Luma::<u8>([0])
            } else {
                Luma::<u8>([0xff])
            }
        });
        self.update_texture(&texture, &image);
        texture
    }
}

// Create depth buffer texture
fn create_depth_texture(device: &Device, size: PhysicalSize<u32>) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("depth_buffer"),
        size: Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
