use crate::{
    event::CustomEvent,
    graphics::Graphics,
    image_cache::ImageCache,
    input::Input,
    rect::Rect,
    widgets::{Root, SharedWidget},
};
use glam::camera::lh::proj::directx::orthographic;
use std::sync::Arc;
use wgpu::{
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, StoreOp, SurfaceError, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

mod atlas;
pub mod axis;
pub mod buffer;
mod color;
pub mod event;
pub mod font;
pub mod font_sdf;
pub mod graphics;
pub mod image_cache;
pub mod input;
pub mod painter;
pub mod quad;
pub mod rect;
pub mod texture_manager;
pub mod theme;
pub mod uid;
mod util;
mod vertex;
pub mod widgets;

pub use atlas::{Atlas, TextureAtlas};
pub use color::Color;
pub use vertex::Vertex;

/// Context for running a widget as an application.
///
/// ```no_run
/// # use piment::widgets::Button;
/// # use piment::App;
/// # use piment::widgets::Share;
/// let widget = Button::new("Hello world!").shared();
/// let app = App::new("Hello").run(widget);
/// ```
pub struct App {
    event_loop: EventLoop<CustomEvent>,
    proxy: EventLoopProxy<CustomEvent>,
    images: ImageCache,
    window_attributes: WindowAttributes,
}

impl App {
    /// Creates the context for running an application.
    ///
    /// `title` is shown in the window title bar.
    pub fn new(title: &str) -> Self {
        // Note: we made title arg mandatory because it is quite awkward when you get a default
        // name in the title bar.
        let event_loop: EventLoop<CustomEvent> = EventLoop::with_user_event().build().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        let proxy = event_loop.create_proxy();
        let images = ImageCache::new(8 * 1024 * 1024 * 1024); // 8 Gb cache
        Self {
            event_loop,
            proxy,
            images,
            window_attributes: Window::default_attributes().with_title(title),
        }
    }

    /// Whether the window starts maximized. Defaults to `false`.
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.window_attributes = self.window_attributes.with_maximized(maximized);
        self
    }

    /// Runs the event loop to display and run a widget.
    /// This method returns when the window is closed.
    pub fn run(self, widget: SharedWidget) {
        let mut state = AppState::new(self.proxy, self.images, self.window_attributes, widget);
        self.event_loop.run_app(&mut state).unwrap();
    }

    /// Returns a reference to the event loop proxy. Can be cloned and shared to threads that may
    /// need to send events to the main event loop, for instance when a process has finished.
    pub fn proxy(&self) -> &EventLoopProxy<CustomEvent> {
        &self.proxy
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
    /// #             core: WidgetCore::new(),
    /// #             images,
    /// #         }
    /// #     }
    /// # }
    /// # impl Widget for SomeWidget {
    /// #     impl_widget_core!();
    /// # }
    /// let app = App::new("Example");
    /// let widget = SomeWidget::new(app.images().clone()).shared();
    /// app.run(widget);
    /// ```
    pub fn images(&self) -> &ImageCache {
        &self.images
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new("")
    }
}

/// Structure needed to run the event loop and handle the events.
struct AppState {
    proxy: EventLoopProxy<CustomEvent>,
    images: ImageCache,
    window_attributes: WindowAttributes,
    graphics: Option<Graphics>,
    input: Input,
    root: Root,
    /// Latest window size received from winit and not yet applied.
    /// Reconfiguring the surface is slow (up to tens of milliseconds), and dragging the window
    /// border emits resize events faster than that. Applying each of them would starve redraws
    /// until the mouse button is released, so only the latest size is applied, once per frame.
    pending_size: Option<PhysicalSize<u32>>,
    /// Set when a widget has requested an animation frame during the latest presented frame, so
    /// that widgets are updated before the next one is rendered.
    animation_pending: bool,
}

impl AppState {
    pub fn new(
        proxy: EventLoopProxy<CustomEvent>,
        images: ImageCache,
        window_attributes: WindowAttributes,
        widget: SharedWidget,
    ) -> Self {
        Self {
            proxy,
            images,
            window_attributes,
            graphics: None,
            input: Input::new(),
            root: Root::new(widget),
            pending_size: None,
            animation_pending: false,
        }
    }

    fn render(&mut self) {
        let graphics = self.graphics.as_mut().unwrap();

        if let Some(size) = self.pending_size.take() {
            graphics.resize(size);
            self.root.layout(
                graphics,
                Rect::new(0.0, 0.0, size.width as f32, size.height as f32),
            );
        }

        // The window may have been resized again since the surface was configured, which happens
        // often while the window border is dragged. In that case the frame is skipped and the
        // surface reconfigured on the next one.
        let surface_texture = match graphics.surface.get_current_texture() {
            Ok(surface_texture) => surface_texture,
            Err(SurfaceError::Outdated | SurfaceError::Lost) => {
                self.pending_size = Some(graphics.window.inner_size());
                graphics.window.request_redraw();
                return;
            }
            Err(SurfaceError::Timeout) => {
                graphics.window.request_redraw();
                return;
            }
            Err(e) => panic!("Failed to acquire surface texture: {e}"),
        };

        // Ask all widgets to render themselves.
        self.root.render(graphics);

        // Transfer buffers to the GPU before any draw call.
        graphics.painter.prepare_render(&graphics.queue);

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

        graphics.painter.projection_matrix = orthographic(
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

        if self.root.animation_request {
            self.root.animation_request = false;
            self.animation_pending = true;
            graphics.window.request_redraw();
        }
    }
}

impl ApplicationHandler<CustomEvent> for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // The window is shown only once sized, as the minimum size is known only after the
        // graphics are created.
        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes.clone().with_visible(false))
                .unwrap(),
        );
        let mut graphics = Graphics::new(window.clone(), self.proxy.clone(), self.images.clone());

        // A null dimension would make the surface configuration invalid.
        let minimum_size = self.root.widget.borrow_mut().minimum_size(&mut graphics);
        let minimum_size = PhysicalSize::new(
            minimum_size.x.ceil().max(1.0) as u32,
            minimum_size.y.ceil().max(1.0) as u32,
        );
        window.set_min_inner_size(Some(minimum_size));
        if !self.window_attributes.maximized
            && let Some(size) = window.request_inner_size(minimum_size)
        {
            self.pending_size = Some(size);
        }
        window.set_visible(self.window_attributes.visible);

        self.root.update(&mut graphics, &self.input);
        if self.root.exit_request {
            event_loop.exit();
        }
        self.graphics = Some(graphics);
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
                if self.animation_pending {
                    self.animation_pending = false;
                    self.root.update(
                        self.graphics.as_mut().unwrap(),
                        &self.input.without_events(),
                    );
                    if self.root.exit_request {
                        event_loop.exit();
                        return;
                    }
                }
                self.render();
            }
            WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. } => {
                self.root
                    .update(self.graphics.as_mut().unwrap(), &self.input);
                if self.root.exit_request {
                    event_loop.exit();
                }
                self.graphics.as_ref().unwrap().window.request_redraw();
            }
            WindowEvent::Resized(size) => {
                self.pending_size = Some(size);
                self.graphics.as_ref().unwrap().window.request_redraw();
            }
            _ => {}
        }
    }

    /// Handles custom events sent by winit proxies, usually from threads to wake-up and notify the
    /// main loop.
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: CustomEvent) {
        let CustomEvent::Signal((signal, listener)) = event;
        if let Some(graphics) = &mut self.graphics {
            self.root.signal(graphics, signal, listener);
        }
        if self.root.exit_request {
            event_loop.exit();
        }
        self.graphics.as_ref().unwrap().window.request_redraw();
    }
}
