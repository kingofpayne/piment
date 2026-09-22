use crate::{
    graphics::Graphics,
    input::Input,
    rect::Rect,
    widgets::{Root, SharedWidget},
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
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod axis;
pub mod buffer;
pub mod color;
pub mod font;
pub mod graphics;
pub mod input;
pub mod painter;
pub mod quad;
pub mod rect;
pub mod theme;
pub mod uid;
pub mod vertex;
pub mod widgets;

struct App {
    graphics: Option<Graphics>,
    input: Input,
    root: Root,
}

impl App {
    fn new(widget: SharedWidget) -> Self {
        Self {
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        self.graphics = Some(Graphics::new(window.clone()));
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
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
}

pub fn run_widget(widget: SharedWidget) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new(widget);
    event_loop.run_app(&mut app).unwrap();
}
