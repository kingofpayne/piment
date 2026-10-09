use super::font::TextHorizontalAlign;
use crate::buffer::DynamicBuffer;
use crate::color::Color;
use crate::font::Font;
use crate::font::TextLayout;
use crate::font_sdf::{FontSdf, OUTLINE_WIDTH, SHADOW_SIGMA_PER_SIZE, SPREAD};
use crate::rect::IRect;
use crate::rect::Rect;
use crate::vertex::Vertex;
use glam::{Mat4, UVec2, Vec2, Vec4, vec4};
use std::{borrow::Cow, collections::BTreeMap, ops::Range, path::PathBuf};
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
    BlendOperation, BlendState, BufferBindingType, BufferSize, BufferUsages, ColorTargetState,
    ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FilterMode,
    FragmentState, IndexFormat, MultisampleState, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPass, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor,
    ShaderSource, ShaderStages, StencilState, Texture, TextureFormat, TextureViewDescriptor,
    TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};

/// Enables easy drawing in the window using WGPU. Implements basic primitive rendering such as
/// lines, circles, round rectangles, text...
///
/// Shapes are painted by drawing triangles with specific shaders. For instance, a round rectangle
/// is a quad (two triangles) with a shader based on Signed Distance Field rendering. By using
/// shaders, the rendered shapes have antialiasing.
///
/// Shapes to be rendered are accumulated in buffers, and sent to the GPU at the end.
pub struct Painter {
    /// WGPU Device. Required to create new pipelines on the fly.
    device: Device,
    /// Surface format. Required to create new pipelines on the fly.
    target_texture_format: TextureFormat,
    /// For each required rendering configuration we need a different GPU pipeline.
    /// Creating a pipeline is an expensive operation (especially due to shader compilation and
    /// validation), so we prefer creating once and caching it there.
    pipelines: BTreeMap<PipelineConfig, RenderResources>,
    /// Current chunk being built.
    chunk: Chunk,
    /// List of rendering chunks which have been committed.
    /// Each chunk stores vertices to be rendered, associated textures and shaders, etc.
    chunks: Vec<Chunk>,
    /// All vertices of all chunks
    /// Filled when the widgets are rendering, then copied in `vertex_buffer` for transferring them
    /// to the GPU.
    vertices: Vec<Vertex>,
    /// Vertex buffer used for all primitives to be rendered.
    /// For each frame, vertices are copied in this buffer.
    /// The buffer is reallocated when too small.
    vertex_buffer: DynamicBuffer,
    /// All vertex indices of all chunks.
    /// Filled when the widgets are rendering, then copied in `index_buffer` for transferring them
    /// to the GPU.
    indices: Vec<u32>,
    /// Index buffer used for all primitives to be rendered.
    /// For each frame, indices are copied in this buffer.
    /// The buffer is reallocated when too small.
    index_buffer: DynamicBuffer,
    /// Surface size.
    pub size: UVec2,
    /// Matrix to transform painter coordinates to screen coordinates.
    /// Usually an orthographic transform created from screen size.
    pub projection_matrix: Mat4,
    /// All new drawing operations are clipped using this region rect.
    pub scissor: IRect,
    /// Loaded fonts that can be used by widgets for rendering.
    /// Fonts are identified by string keys, referenced by [FontStyle::font].
    /// Widgets of the piment library requires the font "main" to be present. This font is populated
    /// automatically.
    /// More fonts may be added for custom widgets.
    pub fonts: BTreeMap<String, Font>,
}

impl Painter {
    const INITIAL_VERTEX_BUFFER_SIZE: u64 = 1024 * size_of::<Vertex>() as u64;
    const INITIAL_INDEX_BUFFER_SIZE: u64 = 1024 * size_of::<u32>() as u64;

    pub fn new(device: &Device, target_texture_format: TextureFormat, size: UVec2) -> Self {
        Self {
            device: device.clone(),
            target_texture_format,
            pipelines: BTreeMap::new(),
            chunk: Default::default(),
            chunks: Vec::new(),
            vertices: Vec::new(),
            vertex_buffer: DynamicBuffer::new(
                device,
                Self::INITIAL_VERTEX_BUFFER_SIZE,
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
            ),
            indices: Vec::new(),
            index_buffer: DynamicBuffer::new(
                device,
                Self::INITIAL_INDEX_BUFFER_SIZE,
                BufferUsages::INDEX | BufferUsages::COPY_DST,
            ),
            size,
            projection_matrix: Mat4::IDENTITY,
            scissor: IRect::new(0, 0, i32::MAX, i32::MAX),
            fonts: BTreeMap::new(),
        }
    }

    /// Commits current chunk and starts a new one.
    fn commit(&mut self) {
        if !self.chunk.indices_range.is_empty() {
            self.chunks.push(self.chunk.clone());
        }
        let start = self.vertices.len();
        self.chunk.vertex_range = start..start;
        let start = self.indices.len();
        self.chunk.indices_range = start..start;
    }

    pub fn begin(&mut self, mut config: ChunkConfig) {
        config.scissor = config.scissor.intersection(self.scissor);
        if config != self.chunk.config {
            self.commit();
            self.chunk.config = config;
        }
    }

    pub fn prepare_render(&mut self, queue: &Queue) {
        // Finish pending chunk
        self.commit();
        // Font glyphs may be created on the fly during widgets rendering.
        // The atlas keeps a dirty flag and need to update the texture to the GPU when the image has
        // been modified.
        for font in self.fonts.values_mut() {
            font.update_texture(&self.device, queue);
        }
        self.vertex_buffer
            .write_slice(&self.device, queue, &self.vertices);
        self.index_buffer
            .write_slice(&self.device, queue, &self.indices);
    }

    /// Renders all stored chunks, and clear them.
    pub fn render(&mut self, pass: &mut RenderPass) {
        // Create missing pipelines.
        // We can't do it in render_chunk because of the borrowing rules.
        for chunk in self.chunks.iter() {
            let resources = self
                .pipelines
                .entry(chunk.config.pipeline.clone())
                .or_insert_with(|| {
                    RenderResources::new(
                        &self.device,
                        self.target_texture_format,
                        &chunk.config.pipeline,
                    )
                });
            pass.set_pipeline(&resources.pipeline);
        }
        for chunk in self.chunks.iter() {
            // Fetch the pipeline corresponding to the rendering configuration.
            // If the pipeline does not exist yet, create it and cache it.
            self.render_chunk(pass, chunk);
        }
        self.chunks.clear();
        self.vertices.clear();
        self.indices.clear();
        self.chunk.vertex_range = 0..0;
        self.chunk.indices_range = 0..0;
    }

    /// Renders one chunk. Don't submit the queue yet.
    fn render_chunk(&self, pass: &mut RenderPass, chunk: &Chunk) {
        // wgpu requires the scissor origin plus size to lie inside the render target.
        let scissor = chunk.config.scissor.intersection(IRect::new(
            0,
            0,
            self.size.x as i32,
            self.size.y as i32,
        ));
        if !scissor.is_sorted() {
            return;
        }

        let uniform_buf = self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(self.projection_matrix.as_ref()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let uniform_buf_fragment_settings = self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(chunk.config.fragment_settings.as_ref()),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let resources = self.pipelines.get(&chunk.config.pipeline).unwrap();
        pass.set_pipeline(&resources.pipeline);

        let mut entries = vec![
            BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: uniform_buf_fragment_settings.as_entire_binding(),
            },
        ];

        // Early declaration of texture_view and sampler to allow them live long enough when they
        // are required.
        let texture_view;
        let sampler;

        if chunk.config.pipeline.texture {
            texture_view = chunk
                .config
                .texture
                .as_ref()
                .unwrap()
                .create_view(&TextureViewDescriptor::default());
            sampler = self.device.create_sampler(&SamplerDescriptor {
                label: None,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: chunk.config.texture_filter_mode,
                min_filter: chunk.config.texture_filter_mode,
                mipmap_filter: FilterMode::Nearest,
                ..Default::default()
            });
            entries.push(BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            });
            entries.push(BindGroupEntry {
                binding: 3,
                resource: BindingResource::Sampler(&sampler),
            });
        }

        pass.set_bind_group(
            0,
            &self.device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &resources.bind_group_layout,
                entries: &entries,
            }),
            &[],
        );

        pass.set_index_buffer(self.index_buffer.inner.slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, self.vertex_buffer.inner.slice(..));
        pass.set_scissor_rect(
            scissor.x1 as u32,
            scissor.y1 as u32,
            scissor.width() as u32,
            scissor.height() as u32,
        );
        pass.draw_indexed(
            chunk.indices_range.start as u32..chunk.indices_range.end as u32,
            0,
            0..1,
        );
    }

    /// Paints a triangle from vertices `a`, `b` and `c`.
    pub fn triangle(&mut self, a: Vertex, b: Vertex, c: Vertex) {
        let i = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[a, b, c]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2]);
        self.chunk.vertex_range.end += 3;
        self.chunk.indices_range.end += 3;
    }

    /// Paints a quad from vertices `a`, `b`, `c` and `d`.
    pub fn quad(&mut self, a: Vertex, b: Vertex, c: Vertex, d: Vertex) {
        let i = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&[a, b, c, d]);
        self.indices
            .extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
        self.chunk.vertex_range.end += 4;
        self.chunk.indices_range.end += 6;
    }

    /// Paint a rectangle at `rect` region.
    pub fn rectangle(&mut self, rect: Rect, colors: [Color; 4]) {
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::ShaderCol)
                .alpha_blending(true),
        );
        self.quad(
            Vertex::from_vec(rect.x1y1()).color1(colors[0]),
            Vertex::from_vec(rect.x1y2()).color1(colors[1]),
            Vertex::from_vec(rect.x2y2()).color1(colors[2]),
            Vertex::from_vec(rect.x2y1()).color1(colors[3]),
        );
    }

    /// Paint a round rectangle at `rect` region.
    ///
    /// `outer_radius` specifies the external radius for each corner, in the following order:
    /// top-left, bottom-left, bottom-right, top-right.
    ///
    /// `inner_radius` specifies the internal radius for each corner. If a radius is 2.0 less than
    /// the outer radius, the line thickness of the rectangle border is 2.0.
    ///
    /// `colors` specified the color of each corner. If all values are the same, the rectangle has
    /// a single color. Passing different values can create gradients.
    pub fn round_rectangle(
        &mut self,
        rect: Rect,
        outer_radius: Vec4,
        inner_radius: Vec4,
        colors: [Color; 4],
    ) {
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::Line)
                .blend(Blend::Alpha)
                .fragment_settings(vec4(2.0, 0.0, 0.0, 0.0)),
        );
        let center = rect.center();
        let geom1 = vec4(center.x, center.y, rect.width() / 2.0, rect.height() / 2.0);
        self.quad(
            Vertex::from_vec(rect.x1y1())
                .color1(colors[0])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x1y2())
                .color1(colors[1])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x2y2())
                .color1(colors[2])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x2y1())
                .color1(colors[3])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
        );
    }

    pub fn round_rectangle_texture(
        &mut self,
        rect: Rect,
        outer_radius: Vec4,
        inner_radius: Vec4,
        colors: [Color; 4],
        texture: Texture,
    ) {
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::LineTex)
                .with_texture(texture)
                .blend(Blend::Alpha)
                .fragment_settings(vec4(2.0, 0.0, 0.0, 0.0)),
        );
        let center = rect.center();
        let geom1 = vec4(center.x, center.y, rect.width() / 2.0, rect.height() / 2.0);
        self.quad(
            Vertex::from_vec(rect.x1y1())
                .uv(0.0, 0.0)
                .color1(colors[0])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x1y2())
                .uv(0.0, 1.0)
                .color1(colors[1])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x2y2())
                .uv(1.0, 1.0)
                .color1(colors[2])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
            Vertex::from_vec(rect.x2y1())
                .uv(1.0, 0.0)
                .color1(colors[3])
                .geom1_vec(geom1)
                .geom2_vec(outer_radius)
                .geom3_vec(inner_radius),
        );
    }

    pub fn circle(&mut self, center: Vec2, outer_radius: f32, inner_radius: f32, color: Color) {
        if outer_radius <= 0.0 {
            return;
        }
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::Line)
                .blend(Blend::Alpha)
                .fragment_settings(vec4(1.0, 0.0, 0.0, 0.0)),
        );
        let z = inner_radius / outer_radius;
        let w = 1.0 / outer_radius; // smoothstep width
        self.quad(
            Vertex::from_xy(center.x - outer_radius, center.y + outer_radius)
                .geom1(-1.0, 1.0, z, w)
                .color1(color),
            Vertex::from_xy(center.x - outer_radius, center.y - outer_radius)
                .geom1(-1.0, -1.0, z, w)
                .color1(color),
            Vertex::from_xy(center.x + outer_radius, center.y - outer_radius)
                .geom1(1.0, -1.0, z, w)
                .color1(color),
            Vertex::from_xy(center.x + outer_radius, center.y + outer_radius)
                .geom1(1.0, 1.0, z, w)
                .color1(color),
        );
    }

    /// Paints a segment without end caps.
    pub fn segment_no_cap(&mut self, a: Vec2, b: Vec2, stroke: Stroke) {
        if a == b {
            return;
        }
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::Line)
                .blend(Blend::Alpha),
        );
        let w = stroke.width + 1.0;
        let u = (b - a).perp().normalize() * w * 0.5;
        self.quad(
            Vertex::from_vec(a + u)
                .geom1(w, 0.0, 0.0, 0.0)
                .color1(stroke.color),
            Vertex::from_vec(a - u)
                .geom1(0.0, w, 0.0, 0.0)
                .color1(stroke.color),
            Vertex::from_vec(b - u)
                .geom1(0.0, w, 0.0, 0.0)
                .color1(stroke.color),
            Vertex::from_vec(b + u)
                .geom1(w, 0.0, 0.0, 0.0)
                .color1(stroke.color),
        );
    }

    pub fn polyline(&mut self, points: &[Vec2], stroke: Stroke) {
        let size = points.len();
        for i in 0..size - 1 {
            let a = points[i];
            let b = points[(i + 1) % size];
            self.segment_no_cap(a, b, stroke);
        }
    }

    pub fn polygon(&mut self, points: &[Vec2], stroke: Stroke) {
        let size = points.len();
        for i in 0..size {
            let a = points[i];
            let b = points[(i + 1) % size];
            self.segment_no_cap(a, b, stroke);
        }
    }

    /// Returns the font registered in [Self::fonts] under `name`.
    ///
    /// # Panics
    ///
    /// Panics if there is no such font.
    pub fn font(&mut self, name: &str) -> &mut Font {
        self.fonts
            .get_mut(name)
            .unwrap_or_else(|| panic!("Font \"{name}\" is not loaded"))
    }

    /// Calculates the width of `text` painted with the font and size of `style`.
    pub fn text_width(&mut self, text: &str, style: &FontStyle) -> f32 {
        self.font(&style.font).text_width(text, style.size)
    }

    /// Builds a [TextLayout] positioning each character of `text` in `rect`, with the font and
    /// size of `style`.
    pub fn layout_text(
        &mut self,
        text: &str,
        rect: Rect,
        align: TextHorizontalAlign,
        style: &FontStyle,
    ) -> TextLayout {
        self.font(&style.font).layout(text, rect, align, style.size)
    }

    /// Paints `text` in `rect`, with the font of `style`. The text is vertically centered in
    /// `rect`, and horizontally placed according to `align`.
    pub fn text(&mut self, text: &str, rect: Rect, align: TextHorizontalAlign, style: &FontStyle) {
        if text.is_empty() {
            return;
        }
        let layout = self.layout_text(text, rect, align, style);
        self.text_layout(&layout, style);
    }

    /// Paints a [TextLayout] built by [Self::layout_text] with the same `style` font.
    pub fn text_layout(&mut self, layout: &TextLayout, style: &FontStyle) {
        if layout.glyphs.is_empty() {
            return;
        }
        let texture = self.font(&style.font).texture().unwrap().clone();
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::Font)
                .blend(Blend::Alpha)
                .with_texture(texture),
        );
        // Shadows, outlines and characters overlap between neighbouring glyphs, so each is drawn
        // in its own pass, back to front.
        // The mask selects which color channel in the font atlas texture is used as an alpha
        // channel for rendering the glyphs: red for characters, green for outlines and blue for
        // shadows.
        if style.shadow_color.alpha() > 0.0 {
            let mask = Color::new_rgb(0.0, 0.0, 1.0);
            self.glyphs(layout, style.shadow_offset, style.shadow_color, mask);
        }
        if style.outline_color.alpha() > 0.0 {
            let mask = Color::new_rgb(0.0, 1.0, 0.0);
            self.glyphs(layout, Vec2::ZERO, style.outline_color, mask);
        }
        if style.color.alpha() > 0.0 {
            let mask = Color::new_rgb(1.0, 0.0, 0.0);
            self.glyphs(layout, Vec2::ZERO, style.color, mask);
        }
    }

    /// Draws every glyph of `layout` moved by `offset`, in `color`, using the atlas channel
    /// selected by `mask` as alpha.
    ///
    /// The drawing color is passed as color1, and the mask as color2.
    fn glyphs(&mut self, layout: &TextLayout, offset: Vec2, color: Color, mask: Color) {
        for g in layout.glyphs.iter() {
            let x1 = g.xy.x1 + offset.x;
            let y1 = g.xy.y1 + offset.y;
            let x2 = g.xy.x2 + offset.x;
            let y2 = g.xy.y2 + offset.y;
            self.quad(
                Vertex::from_xy(x1, y1)
                    .uv(g.uv.x1, g.uv.y1)
                    .color1(color)
                    .color2(mask),
                Vertex::from_xy(x1, y2)
                    .uv(g.uv.x1, g.uv.y2)
                    .color1(color)
                    .color2(mask),
                Vertex::from_xy(x2, y2)
                    .uv(g.uv.x2, g.uv.y2)
                    .color1(color)
                    .color2(mask),
                Vertex::from_xy(x2, y1)
                    .uv(g.uv.x2, g.uv.y1)
                    .color1(color)
                    .color2(mask),
            );
        }
    }

    /// Paints `text` in `rect` with a distance field font, which can be drawn at any
    /// [FontStyle::size].
    pub fn text_sdf(&mut self, font: &FontSdf, text: &str, rect: Rect, style: &FontStyle) {
        if text.is_empty() {
            return;
        }
        let layout = font.layout(text, rect, TextHorizontalAlign::Left, style.size as f32);
        self.text_layout_sdf(font, &layout, style);
    }

    /// Paints a [TextLayout] built by [FontSdf::layout].
    pub fn text_layout_sdf(&mut self, font: &FontSdf, layout: &TextLayout, style: &FontStyle) {
        if layout.glyphs.is_empty() {
            return;
        }
        self.begin(
            ChunkConfig::default()
                .shader(BuiltinShader::FontSdf)
                .blend(Blend::Alpha)
                .with_texture(font.texture().unwrap().clone()),
        );
        // Shadows, outlines and characters overlap between neighbouring glyphs, so each is drawn
        // in its own pass, back to front. Passes only differ by their contour settings, which are
        // per vertex, so they all share the same draw call.
        if style.shadow_color.alpha() > 0.0 {
            // The shadow edge must fade out before the distances clamped at the atlas spread.
            let sigma = SHADOW_SIGMA_PER_SIZE * style.size as f32;
            let softness = 0.5 + 1.5 * sigma;
            self.glyphs_sdf(
                layout,
                style.shadow_offset,
                style.shadow_color,
                0.0,
                softness,
            );
        }
        if style.outline_color.alpha() > 0.0 {
            let dilation = OUTLINE_WIDTH / 2.0;
            self.glyphs_sdf(layout, Vec2::ZERO, style.outline_color, dilation, 0.5);
        }
        if style.color.alpha() > 0.0 {
            self.glyphs_sdf(layout, Vec2::ZERO, style.color, 0.0, 0.5);
        }
    }

    /// Draws every glyph of `layout` moved by `offset`, in `color`, from a distance field atlas.
    ///
    /// The glyphs contour is moved outwards by `dilation` pixels, and `softness` is the half width
    /// of the edge transition, in pixels.
    fn glyphs_sdf(
        &mut self,
        layout: &TextLayout,
        offset: Vec2,
        color: Color,
        dilation: f32,
        softness: f32,
    ) {
        let geom1 = vec4(2.0 * SPREAD as f32, dilation, softness, 0.0);
        for g in layout.glyphs.iter() {
            let x1 = g.xy.x1 + offset.x;
            let y1 = g.xy.y1 + offset.y;
            let x2 = g.xy.x2 + offset.x;
            let y2 = g.xy.y2 + offset.y;
            self.quad(
                Vertex::from_xy(x1, y1)
                    .uv(g.uv.x1, g.uv.y1)
                    .color1(color)
                    .geom1_vec(geom1),
                Vertex::from_xy(x1, y2)
                    .uv(g.uv.x1, g.uv.y2)
                    .color1(color)
                    .geom1_vec(geom1),
                Vertex::from_xy(x2, y2)
                    .uv(g.uv.x2, g.uv.y2)
                    .color1(color)
                    .geom1_vec(geom1),
                Vertex::from_xy(x2, y1)
                    .uv(g.uv.x2, g.uv.y1)
                    .color1(color)
                    .geom1_vec(geom1),
            );
        }
    }
}

/// Every setting for rendering triangles in a particular way.
#[derive(Clone, PartialEq)]
pub struct ChunkConfig {
    /// Texture to be binded.
    texture: Option<Texture>,
    /// Texture filter mode.
    texture_filter_mode: FilterMode,
    /// Extra settings to be passed to the fragment shader.
    fragment_settings: Vec4,
    /// Configuration settings relative to the pipeline itself. This is separated so we can use
    /// this value as a key to grab the correct pipeline to be enabled.
    pipeline: PipelineConfig,
    /// Scissor region.
    /// Maxed by default (no clipping).
    /// Enabling clipping shall be avoided whenever possible, because it reduces the possibility to
    /// merge draw calls.
    scissor: IRect,
}

impl ChunkConfig {
    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self.pipeline.texture = true;
        self
    }

    pub fn with_texture_filter_mode(mut self, value: FilterMode) -> Self {
        self.texture_filter_mode = value;
        self
    }

    pub fn fragment_settings(mut self, value: Vec4) -> Self {
        self.fragment_settings = value;
        self
    }

    pub fn shader(mut self, shader: impl Into<Shader>) -> Self {
        self.pipeline.shader = shader.into();
        self
    }

    pub fn shader_file(self, path: impl Into<PathBuf>) -> Self {
        self.shader(Shader::File(path.into()))
    }

    pub fn shader_source(self, source: impl Into<Cow<'static, str>>) -> Self {
        self.shader(Shader::Source(source.into()))
    }

    pub fn with_depth_buffer(mut self) -> Self {
        self.pipeline.depth_buffer = true;
        self
    }

    pub fn depth_buffer(mut self, value: bool) -> Self {
        self.pipeline.depth_buffer = value;
        self
    }

    pub fn blend(mut self, value: Blend) -> Self {
        self.pipeline.blend = value;
        self
    }

    pub fn alpha_blending(mut self, value: bool) -> Self {
        self.pipeline.blend = if value { Blend::Alpha } else { Blend::None };
        self
    }

    pub fn with_scissor(self, scissor: IRect) -> Self {
        Self { scissor, ..self }
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            texture: Default::default(),
            texture_filter_mode: FilterMode::Linear,
            fragment_settings: Default::default(),
            pipeline: Default::default(),
            scissor: IRect::new(0, 0, i32::MAX, i32::MAX),
        }
    }
}

/// Triangles to be rendered and configuration describing how to render it.
#[derive(Default, Clone)]
struct Chunk {
    /// Vertex range in the global array.
    vertex_range: Range<usize>,
    /// Indices range in the global array.
    indices_range: Range<usize>,
    /// Rendering settings (texture, blending, etc.)
    config: ChunkConfig,
}

/// WGPU resources required to paint using a particular method.
pub struct RenderResources {
    pub bind_group_layout: BindGroupLayout,
    pub pipeline: RenderPipeline,
}

/// Shaders shipped with the library, embedded in the binary.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum BuiltinShader {
    #[default]
    ShaderCol,
    Line,
    LineTex,
    Font,
    FontSdf,
}

/// WGSL shader used by a pipeline.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Shader {
    /// Shader shipped with the library.
    Builtin(BuiltinShader),
    /// WGSL file read when the pipeline is created. Relative paths resolve against the current
    /// working directory.
    File(PathBuf),
    /// WGSL source, e.g. from `include_str!`.
    Source(Cow<'static, str>),
}

impl Default for Shader {
    fn default() -> Self {
        Self::Builtin(BuiltinShader::default())
    }
}

impl From<BuiltinShader> for Shader {
    fn from(value: BuiltinShader) -> Self {
        Self::Builtin(value)
    }
}

impl Shader {
    /// Returns the WGSL source of the shader.
    pub fn wgsl(&self) -> Cow<'static, str> {
        match self {
            Self::Builtin(BuiltinShader::ShaderCol) => {
                include_str!("../shaders/shader-col.wgsl").into()
            }
            Self::Builtin(BuiltinShader::Line) => include_str!("../shaders/line.wgsl").into(),
            Self::Builtin(BuiltinShader::LineTex) => {
                include_str!("../shaders/line-tex.wgsl").into()
            }
            Self::Builtin(BuiltinShader::Font) => include_str!("../shaders/font.wgsl").into(),
            Self::Builtin(BuiltinShader::FontSdf) => {
                include_str!("../shaders/font-sdf.wgsl").into()
            }
            Self::File(path) => std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to load shader source {}: {e}", path.display()))
                .into(),
            Self::Source(source) => source.clone(),
        }
    }
}

#[derive(Default, PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct PipelineConfig {
    /// Use of texture.
    pub texture: bool,
    /// Pipeline shader to be used.
    pub shader: Shader,
    /// Blending mode.
    pub blend: Blend,
    /// Use of Z-buffer.
    pub depth_buffer: bool,
}

impl RenderResources {
    const VERTEX_BUFFER_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Vertex>() as u64,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            // xyz
            VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            // uv
            VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 4 * 3,
                shader_location: 1,
            },
            // color1
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 5,
                shader_location: 2,
            },
            // color2
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 9,
                shader_location: 3,
            },
            // geom1
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 13,
                shader_location: 4,
            },
            // geom2
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 17,
                shader_location: 5,
            },
            // geom3
            VertexAttribute {
                format: VertexFormat::Float32x4,
                offset: 4 * 21,
                shader_location: 6,
            },
        ],
    };

    const MATRIX_BIND_GROUP_LAYOUT_ENTRY: BindGroupLayoutEntry = BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(64),
        },
        count: None,
    };

    /// Creates a pipeline for the given rendering settings.
    pub fn new(
        device: &Device,
        target_texture_format: TextureFormat,
        config: &PipelineConfig,
    ) -> Self {
        let mut entries = vec![
            Self::MATRIX_BIND_GROUP_LAYOUT_ENTRY,
            // Extra settings for shaders, passed as a vec4f.
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(16),
                },
                count: None,
            },
        ];
        if config.texture {
            entries.push(BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            entries.push(BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            });
        }
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffer_layouts = [Self::VERTEX_BUFFER_LAYOUT];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(config.shader.wgsl()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffer_layouts,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: target_texture_format,
                    blend: match config.blend {
                        Blend::None => None,
                        Blend::Alpha => Some(BlendState::ALPHA_BLENDING),
                        Blend::Additive => Some(BlendState {
                            color: BlendComponent {
                                src_factor: BlendFactor::SrcAlpha,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                            alpha: BlendComponent::REPLACE,
                        }),
                    },
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: config.depth_buffer,
                depth_compare: if config.depth_buffer {
                    CompareFunction::Less
                } else {
                    CompareFunction::Always
                },
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Blend {
    #[default]
    None,
    Alpha,
    Additive,
}

#[derive(Copy, Clone)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

impl Stroke {
    pub fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
}

/// Default text size, in pixels.
pub const DEFAULT_FONT_SIZE: i32 = 11;

#[derive(Clone)]
pub struct FontStyle {
    /// Font identifier, as stored in [Painter::fonts].
    /// By default "main" to use the main font.
    pub font: String,
    /// Text color.
    pub color: Color,
    /// Outline drawing color. Default is transparent.
    pub outline_color: Color,
    /// Shadow drawing color. Default is transparent.
    pub shadow_color: Color,
    /// Shadow offset from the text, in pixels.
    pub shadow_offset: Vec2,
    /// Text size. Must be one of the sizes built in the font atlas, except for [FontSdf] which
    /// accepts any size.
    pub size: i32,
}

impl FontStyle {
    pub fn new() -> Self {
        Self {
            font: "main".into(),
            color: Color::WHITE,
            outline_color: Color::BLACK_TRANSPARENT,
            shadow_color: Color::BLACK_TRANSPARENT,
            shadow_offset: Vec2::ZERO,
            size: DEFAULT_FONT_SIZE,
        }
    }

    /// Sets the font identifier, as stored in [Painter::fonts].
    pub fn font(mut self, name: impl Into<String>) -> Self {
        self.font = name.into();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn outline_color(mut self, color: Color) -> Self {
        self.outline_color = color;
        self
    }

    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    pub fn shadow_offset(mut self, offset: Vec2) -> Self {
        self.shadow_offset = offset;
        self
    }

    pub fn size(mut self, size: i32) -> Self {
        self.size = size;
        self
    }
}

impl Default for FontStyle {
    fn default() -> Self {
        Self::new()
    }
}
