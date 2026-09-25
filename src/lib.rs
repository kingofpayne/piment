use crate::{
    event::CustomEvent,
    graphics::Graphics,
    image_cache::ImageCache,
    input::Input,
    rect::Rect,
    widgets::{Panel, Root, Share, SharedWidget},
};
use glam::Mat4;
use std::sync::Arc;
use wgpu::{
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::Window,
};

pub mod axis;
pub mod buffer;
pub mod color;
pub mod event;
pub mod font;
pub mod graphics;
pub mod image_cache;
pub mod input;
pub mod painter;
pub mod quad;
pub mod rect;
pub mod texture_manager;
pub mod theme;
pub mod uid;
pub mod vertex;
pub mod widgets;

/// Context for running a widget as an application.
///
/// ```
/// # use piment::widgets::Button;
/// # use piment::App;
/// # use piment::widgets::Share;
/// let widget = Button::new("Hello world!").shared();
/// let app = App::new().run(widget);
/// ```
pub struct App {
    event_loop: EventLoop<CustomEvent>,
    proxy: EventLoopProxy<CustomEvent>,
    images: ImageCache,
}

impl App {
    /// Creates the context for running an application.
    pub fn new() -> Self {
        let event_loop: EventLoop<CustomEvent> = EventLoop::with_user_event().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        let proxy = event_loop.create_proxy();
        let images = ImageCache::new(8 * 1024 * 1024 * 1024); // 8 Gb cache
        Self {
            event_loop,
            proxy,
            images,
        }
    }

    /// Runs the event loop to display and run a widget.
    /// This method returns when the window is closed.
    pub fn run(self, widget: SharedWidget) {
        let mut state = AppState::new(self.proxy, self.images, widget);
        self.event_loop.run_app(&mut state).unwrap();
    }

    /// Returns a reference to the image cache.
    /// This cache is used when loading textures and shared by [Graphics]. It can be cloned for
    /// sharing with widgets or threads that may need to load and access images.
    ///
    /// The following example shows how to share the image cache created by App with a widget.
    ///
    /// ```no_run
    /// # use piment::App;
    /// # use piment::image_cache::ImageCache;
    /// # use piment::impl_widget_core;
    /// # use piment::widgets::{Share, Widget, WidgetCore};
    /// # struct SomeWidget {
    /// #     core: WidgetCore,
    /// #     images: ImageCache,
    /// # }
    /// # impl SomeWidget {
    /// #     fn new(images: ImageCache) -> Self {
    /// #         Self {
    /// #             core: WidgetCore::new("SomeWidget"),
    /// #             images,
    /// #         }
    /// #     }
    /// # }
    /// # impl Widget for SomeWidget {
    /// #     impl_widget_core!();
    /// # }
    /// let app = App::new();
    /// let widget = SomeWidget::new(app.images().clone()).shared();
    /// app.run(widget);
    /// ```
    pub fn images(&self) -> &ImageCache {
        &self.images
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

struct AppState {
    proxy: EventLoopProxy<CustomEvent>,
    images: ImageCache,
    graphics: Option<Graphics>,
    input: Input,
    root: Root,
}

impl AppState {
    pub fn new(
        proxy: EventLoopProxy<CustomEvent>,
        images: ImageCache,
        widget: SharedWidget,
    ) -> Self {
        Self {
            proxy,
            images,
            graphics: None,
            input: Input::new(),
            root: Root::new(widget),
        }
    }

    fn render(&mut self) {
        let graphics = self.graphics.as_mut().unwrap();

        // Ask all widgets to render themselves.
        self.root.render(graphics);

        // Transfer buffers to the GPU before any draw call.
        graphics.painter.prepare_render(&graphics.queue);

        let surface_texture = graphics.surface.get_current_texture().unwrap();
        let texture_view = surface_texture
            .texture
            .clone()
            .create_view(&TextureViewDescriptor {
                format: Some(graphics.surface_format.remove_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = graphics.device.create_command_encoder(&Default::default());
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &graphics
                    .texture_depth
                    .create_view(&TextureViewDescriptor::default()),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        graphics.painter.projection_matrix = Mat4::orthographic_lh(
            0.0,
            graphics.size.width as f32,
            graphics.size.height as f32,
            0.0,
            0.0,
            1.0,
        );

        graphics.painter.render(&mut render_pass);
        drop(render_pass);
        graphics.queue.submit([encoder.finish()]);
        graphics.window.pre_present_notify();
        surface_texture.present();
    }
}

impl ApplicationHandler<CustomEvent> for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.graphics = Some(Graphics::new(
            window.clone(),
            self.proxy.clone(),
            self.images.clone(),
        ));
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.input.update_from_winit_event(&event);
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. } => {
                self.root
                    .update(self.graphics.as_mut().unwrap(), &self.input);
                self.graphics.as_ref().unwrap().window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                self.graphics.as_mut().unwrap().resize(size);
                self.root.layout(
                    self.graphics.as_mut().unwrap(),
                    Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
                );
            }
            _ => {}
        }
    }

    /// Handles custom events sent by winit proxies, usually from threads to wake-up and notify the
    /// main loop.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        let CustomEvent::Signal((signal, listener)) = event;
        if let Some(graphics) = &mut self.graphics {
            self.root.signal(graphics, signal, listener);
        }
        self.graphics.as_ref().unwrap().window.request_redraw();
    }
}
