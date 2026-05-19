use std::mem::size_of;
use std::ops::Range;

use glyph_brush::{ab_glyph::PxScale, BrushAction, BrushError, Section, Text};

use super::batch::RenderBatch;
use super::vertex::UiVertex;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::styles::Rect;
use crate::wgpu_backend::pipelines::custom_ui::{
    self, make_texture_bind_group, UiPipelines, UiUniforms,
};

/// CPU-side glyph quad emitted from `GlyphBrush::process_queued`. Stored on
/// the renderer so a `BrushAction::ReDraw` can re-emit the previous frame's
/// glyphs without re-shaping the text.
#[derive(Clone, Copy, Debug)]
pub struct UiGlyph {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PipelineKind {
    Solid,
    AlphaMask,
}

/// One recorded draw range. Built by `commit_batch` when state changes
/// (texture id, pipeline kind, or scissor rect).
#[derive(Clone, Debug)]
struct DrawCmd {
    index_range: Range<u32>,
    texture_id: u32,
    pipeline_kind: PipelineKind,
    scissor: Option<[u32; 4]>,
}

/// Registry of per-texture bind groups, keyed by a dense `u32` id.
///
/// Widgets and RON files identify textures with a plain
/// `u32` (originally an opengl `glGenTextures` name). wgpu has no equivalent. every
/// draw needs a `BindGroup` built ahead of time from a `TextureView` +
/// `Sampler` + the pipeline's group-1 layout. This registry owns one
/// `BindGroup` per texture and lets the renderer translate
/// `texture_id -> &BindGroup` at draw time.
///
/// Slot conventions:
/// - id `0`: 1x1 white pixel, so the same `solid` pipeline handles flat
///   fills (color * white = color).
/// - id `1`: font glyph atlas, used by the `alpha_mask` pipeline.
///   `glyph_brush`'s `TextureTooSmall` resize calls `replace(1, ...)` so
///   the id stays stable even when the underlying texture is recreated.
pub struct UiTextureRegistry {
    layout: wgpu::BindGroupLayout,
    entries: Vec<wgpu::BindGroup>,
}

impl UiTextureRegistry {
    fn new(layout: wgpu::BindGroupLayout) -> Self {
        Self {
            layout,
            entries: Vec::new(),
        }
    }

    /// Mint a new id and bind the given `view`/`sampler` to it.
    pub fn register(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> u32 {
        let id = self.entries.len() as u32;
        let bg = make_texture_bind_group(
            device,
            &self.layout,
            view,
            sampler,
            Some("ui_texture_bind_group"),
        );
        self.entries.push(bg);
        id
    }

    /// Rebind an existing id to a new view/sampler. Used when the font atlas
    /// is recreated on resize so the `font_texture_id` stays stable.
    pub fn replace(
        &mut self,
        id: u32,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        let bg = make_texture_bind_group(
            device,
            &self.layout,
            view,
            sampler,
            Some("ui_texture_bind_group"),
        );
        self.entries[id as usize] = bg;
    }

    fn bind_group(&self, id: u32) -> &wgpu::BindGroup {
        &self.entries[id as usize]
    }
}

const INITIAL_VERTEX_CAP: u64 = 16_384;
const INITIAL_INDEX_CAP: u64 = 32_768;
const INITIAL_FONT_ATLAS: u32 = 256;

pub struct UiRenderer {
    pipelines: UiPipelines,
    textures: UiTextureRegistry,

    screen_width: f32,
    screen_height: f32,
    screen_size_dirty: bool,

    // GPU buffers — recreated when capacity is exceeded.
    vertex_buffer: wgpu::Buffer,
    vertex_buffer_cap: u64,
    index_buffer: wgpu::Buffer,
    index_buffer_cap: u64,

    /// Frame-wide accumulating batch. Cleared on `begin()`.
    batch: RenderBatch,
    /// Snapshots of consecutive ranges of `batch.indices`, one per draw
    /// state change (texture / pipeline / scissor).
    draws: Vec<DrawCmd>,
    /// Index count at the start of the next uncommitted range.
    last_index_count: u32,

    // Current draw state used to detect transitions.
    active_texture: u32,
    active_pipeline: PipelineKind,

    /// Scissor rects in **top-left** pixel space (x, y, w, h), matching
    /// `wgpu::RenderPass::set_scissor_rect`. The old GL bottom-left flip is
    /// gone.
    scissor_stack: Vec<[u32; 4]>,

    // Reserved texture ids (see `UiTextureRegistry`).
    white_texture_id: u32,
    font_texture_id: u32,

    // Font glyph atlas — single R8Unorm texture owned by the renderer so
    // we can resize it (recreate at a new size) when `glyph_brush` asks for
    // more space. The bind group inside `textures` holds the matching view;
    // we keep the `Texture` handle here because `queue.write_texture` needs
    // `&wgpu::Texture`, not a view.
    font_atlas_texture: wgpu::Texture,
    font_atlas_sampler: wgpu::Sampler,
    font_atlas_width: u32,
    font_atlas_height: u32,

    // Side-channel for `process_text_batch`: collected during `end()` then
    // applied against the wgpu queue/device inside `prepare()`. The deferral
    // is what lets `end()` keep a `(&mut self, &mut FontSystem)` signature
    // (no device/queue access at text-shape time).
    pending_atlas_resize: Option<(u32, u32)>,
    pending_atlas_uploads: Vec<AtlasUpload>,

    queued_text: Vec<QueuedText>,
    cached_glyphs: Vec<UiGlyph>,

    overlay_rects: Vec<(Rect, [f32; 4], f32)>,
    overlay_text: Vec<OverlayText>,

    default_font_family: Option<String>,
}

#[derive(Clone, Debug)]
struct QueuedText {
    text: String,
    x: f32,
    y: f32,
    font_size: f32,
    color: [f32; 4],
    font_family: Option<String>,
    scissor: Option<[u32; 4]>,
}

#[derive(Clone, Debug)]
struct OverlayText {
    text: String,
    x: f32,
    y: f32,
    font_size: f32,
    color: [f32; 4],
    font_family: Option<String>,
}

#[derive(Clone, Debug)]
struct AtlasUpload {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    data: Vec<u8>,
}

impl UiRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let pipelines = custom_ui::build(device, surface_format);

        let mut textures = UiTextureRegistry::new(pipelines.texture_layout.clone());

        // Reserved slot 0: 1x1 white pixel. `draw_rect` paths bind id 0 so
        // the solid pipeline handles flat fills as `color * white == color`.
        let white_texture =
            create_solid_rgba_texture(device, queue, [255, 255, 255, 255], "ui_white_pixel");
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let white_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui_white_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let white_texture_id = textures.register(device, &white_view, &white_sampler);

        // Reserved slot 1: font glyph atlas (R8Unorm). Starts as a small
        // placeholder; `process_text_batch` may schedule a resize on the
        // first frame that `glyph_brush` reports `TextureTooSmall`.
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui_font_atlas_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let (font_atlas_texture, font_view) =
            create_font_atlas(device, INITIAL_FONT_ATLAS, INITIAL_FONT_ATLAS);
        let font_texture_id = textures.register(device, &font_view, &font_sampler);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_vertex_buffer"),
            size: INITIAL_VERTEX_CAP * size_of::<UiVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_index_buffer"),
            size: INITIAL_INDEX_CAP * size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipelines,
            textures,
            screen_width: 1920.0,
            screen_height: 1080.0,
            screen_size_dirty: true,

            vertex_buffer,
            vertex_buffer_cap: INITIAL_VERTEX_CAP,
            index_buffer,
            index_buffer_cap: INITIAL_INDEX_CAP,

            batch: RenderBatch::new(),
            draws: Vec::new(),
            last_index_count: 0,

            active_texture: white_texture_id,
            active_pipeline: PipelineKind::Solid,

            scissor_stack: Vec::new(),

            white_texture_id,
            font_texture_id,

            font_atlas_texture,
            font_atlas_sampler: font_sampler,
            font_atlas_width: INITIAL_FONT_ATLAS,
            font_atlas_height: INITIAL_FONT_ATLAS,

            pending_atlas_resize: None,
            pending_atlas_uploads: Vec::new(),

            queued_text: Vec::new(),
            cached_glyphs: Vec::new(),

            overlay_rects: Vec::new(),
            overlay_text: Vec::new(),

            default_font_family: None,
        }
    }

    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        if (self.screen_width - width).abs() > f32::EPSILON
            || (self.screen_height - height).abs() > f32::EPSILON
        {
            self.screen_width = width;
            self.screen_height = height;
            self.screen_size_dirty = true;
        }
    }

    pub fn set_default_font_family(&mut self, font_family: Option<String>) {
        self.default_font_family = font_family;
    }

    /// Register an external texture (e.g. portrait FBO) and return the `u32`
    /// id widgets like `TextureRect` should assign to `texture_id`.
    pub fn register_texture(
        &mut self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> u32 {
        self.textures.register(device, view, sampler)
    }

    pub fn begin(&mut self) {
        self.batch.clear();
        self.draws.clear();
        self.last_index_count = 0;
        self.queued_text.clear();
        self.overlay_rects.clear();
        self.overlay_text.clear();
        self.scissor_stack.clear();
        self.active_texture = self.white_texture_id;
        self.active_pipeline = PipelineKind::Solid;
    }

    pub fn push_scissor(&mut self, rect: Rect) {
        self.commit_batch();

        let new_scissor = clip_rect_to_screen(rect, self.screen_width, self.screen_height);
        let intersected = match self.scissor_stack.last() {
            Some(curr) => intersect_rects(*curr, new_scissor),
            None => new_scissor,
        };
        self.scissor_stack.push(intersected);
    }

    pub fn pop_scissor(&mut self) {
        self.commit_batch();
        self.scissor_stack.pop();
    }

    pub fn draw_rect(&mut self, rect: Rect, color: [f32; 4], border_radius: f32) {
        self.ensure_state(self.white_texture_id, PipelineKind::Solid);
        let clamped_radius = border_radius
            .min(rect.width.min(rect.height) * 0.5)
            .max(0.0);
        self.batch.push_rect(rect, color, clamped_radius);
    }

    pub fn draw_diamond(&mut self, rect: Rect, color: [f32; 4]) {
        self.ensure_state(self.white_texture_id, PipelineKind::Solid);

        let cx = rect.x + rect.width / 2.0;
        // Lift by a few pixels so the diamond sits above the divider line
        // (matches the GL renderer's offset).
        let cy = rect.y + rect.height / 2.0 - 4.0;
        let size = rect.width.min(rect.height) / 2.0;

        let top = (cx, cy - size);
        let right = (cx + size, cy);
        let bottom = (cx, cy + size);
        let left = (cx - size, cy);

        self.batch
            .push_triangle([top.0, top.1], [right.0, right.1], [cx, cy], color);
        self.batch
            .push_triangle([right.0, right.1], [bottom.0, bottom.1], [cx, cy], color);
        self.batch
            .push_triangle([bottom.0, bottom.1], [left.0, left.1], [cx, cy], color);
        self.batch
            .push_triangle([left.0, left.1], [top.0, top.1], [cx, cy], color);
    }

    pub fn draw_textured_rect(&mut self, rect: Rect, texture_id: u32, color: Option<[f32; 4]>) {
        self.draw_textured_rect_ex(rect, texture_id, color, false);
    }

    /// `flip_v` is kept for source compatibility with the OpenGL renderer
    /// (FBO textures used to need a vertical flip). wgpu render targets are
    /// already top-left, so the flag is intentionally ignored.
    pub fn draw_textured_rect_ex(
        &mut self,
        rect: Rect,
        texture_id: u32,
        color: Option<[f32; 4]>,
        _flip_v: bool,
    ) {
        self.ensure_state(texture_id, PipelineKind::Solid);
        let color = color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
        self.batch
            .push_textured_rect(rect, [0.0, 0.0, 1.0, 1.0], color);
    }

    pub fn draw_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
        font_family: Option<&str>,
    ) {
        let scissor = self.scissor_stack.last().copied();
        self.queued_text.push(QueuedText {
            text: text.to_string(),
            x,
            y,
            font_size,
            color,
            font_family: font_family.map(|s| s.to_string()),
            scissor,
        });
    }

    pub fn draw_overlay_rect(&mut self, rect: Rect, color: [f32; 4], border_radius: f32) {
        self.overlay_rects.push((rect, color, border_radius));
    }

    pub fn draw_overlay_text(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: [f32; 4],
        font_family: Option<&str>,
    ) {
        self.overlay_text.push(OverlayText {
            text: text.to_string(),
            x,
            y,
            font_size,
            color,
            font_family: font_family.map(|s| s.to_string()),
        });
    }

    /// Finalize CPU-side work: run `glyph_brush` for queued text, build the
    /// overlay layer, and snapshot the remaining batch into a [`DrawCmd`].
    /// Does **not** touch the GPU — see [`UiRenderer::prepare`].
    pub fn end(&mut self, font_system: &mut FontSystem) {
        self.commit_batch();

        // ==========================================
        // TEXT PASS
        // ==========================================
        if !self.queued_text.is_empty() {
            let mut sorted: Vec<QueuedText> = std::mem::take(&mut self.queued_text);
            sorted.sort_by_key(|t| t.scissor);

            let mut current_scissor = sorted[0].scissor;
            let mut batch_open = false;

            for t in sorted {
                if t.scissor != current_scissor {
                    if batch_open {
                        self.process_text_batch(font_system, current_scissor);
                    }
                    current_scissor = t.scissor;
                }

                let effective_font = t.font_family.or_else(|| self.default_font_family.clone());
                let font_id = font_system.get_font_id(effective_font.as_deref());
                let scale = font_system.get_font_scale(effective_font.as_deref());
                let scaled_size = t.font_size * scale;

                font_system.glyph_brush.queue(Section {
                    screen_position: (t.x, t.y),
                    bounds: (self.screen_width, self.screen_height),
                    text: vec![Text::new(&t.text)
                        .with_scale(PxScale::from(scaled_size))
                        .with_color(t.color)
                        .with_font_id(font_id)],
                    ..Section::default()
                });
                batch_open = true;
            }

            if batch_open {
                self.process_text_batch(font_system, current_scissor);
            }
        }

        // ==========================================
        // OVERLAY PASS — drawn on top, ignoring any active scissor.
        // ==========================================
        let saved_scissor = std::mem::take(&mut self.scissor_stack);
        self.commit_batch();

        if !self.overlay_rects.is_empty() {
            let rects: Vec<_> = std::mem::take(&mut self.overlay_rects);
            for (rect, color, border_radius) in rects {
                self.draw_rect(rect, color, border_radius);
            }
            self.commit_batch();
        }

        if !self.overlay_text.is_empty() {
            let texts: Vec<OverlayText> = std::mem::take(&mut self.overlay_text);
            for t in texts {
                let effective_font = t.font_family.or_else(|| self.default_font_family.clone());
                let font_id = font_system.get_font_id(effective_font.as_deref());
                let scale = font_system.get_font_scale(effective_font.as_deref());
                let scaled_size = t.font_size * scale;

                font_system.glyph_brush.queue(Section {
                    screen_position: (t.x, t.y),
                    bounds: (self.screen_width, self.screen_height),
                    text: vec![Text::new(&t.text)
                        .with_scale(PxScale::from(scaled_size))
                        .with_color(t.color)
                        .with_font_id(font_id)],
                    ..Section::default()
                });
            }
            self.process_text_batch(font_system, None);
        }

        self.scissor_stack = saved_scissor;
        self.commit_batch();
    }

    /// Upload everything to the GPU. Call once per frame between `end()`
    /// and `render()`.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.screen_size_dirty {
            queue.write_buffer(
                &self.pipelines.screen_uniform_buffer,
                0,
                bytemuck::bytes_of(&UiUniforms {
                    screen_size: [self.screen_width, self.screen_height],
                    _pad: [0.0, 0.0],
                }),
            );
            self.screen_size_dirty = false;
        }

        // Apply any pending font-atlas state collected during `end()`.
        // Resize first (creates a new texture + bind group), then upload
        // subimages into the now-current atlas.
        if let Some((w, h)) = self.pending_atlas_resize.take() {
            let (texture, view) = create_font_atlas(device, w, h);
            self.textures.replace(
                self.font_texture_id,
                device,
                &view,
                &self.font_atlas_sampler,
            );
            self.font_atlas_texture = texture;
            self.font_atlas_width = w;
            self.font_atlas_height = h;
        }
        if !self.pending_atlas_uploads.is_empty() {
            let uploads = std::mem::take(&mut self.pending_atlas_uploads);
            for u in uploads {
                if u.w <= 0 || u.h <= 0 {
                    continue;
                }
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.font_atlas_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: u.x as u32,
                            y: u.y as u32,
                            z: 0,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &u.data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(u.w as u32),
                        rows_per_image: Some(u.h as u32),
                    },
                    wgpu::Extent3d {
                        width: u.w as u32,
                        height: u.h as u32,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let vertex_bytes = bytemuck::cast_slice::<UiVertex, u8>(&self.batch.vertices);
        let index_bytes = bytemuck::cast_slice::<u32, u8>(&self.batch.indices);

        if !vertex_bytes.is_empty() {
            let required_verts = self.batch.vertices.len() as u64;
            if required_verts > self.vertex_buffer_cap {
                let new_cap = required_verts.next_power_of_two().max(INITIAL_VERTEX_CAP);
                self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ui_vertex_buffer"),
                    size: new_cap * size_of::<UiVertex>() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.vertex_buffer_cap = new_cap;
            }
            queue.write_buffer(&self.vertex_buffer, 0, vertex_bytes);
        }

        if !index_bytes.is_empty() {
            let required_indices = self.batch.indices.len() as u64;
            if required_indices > self.index_buffer_cap {
                let new_cap = required_indices.next_power_of_two().max(INITIAL_INDEX_CAP);
                self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("ui_index_buffer"),
                    size: new_cap * size_of::<u32>() as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.index_buffer_cap = new_cap;
            }
            queue.write_buffer(&self.index_buffer, 0, index_bytes);
        }
    }

    /// Issue the recorded draws into an already-open render pass. Called
    /// from the overlay closure passed to `Renderer::render_world_with_overlay`.
    pub fn render<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        if self.draws.is_empty() || self.batch.indices.is_empty() {
            return;
        }

        rpass.set_bind_group(0, &self.pipelines.screen_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        let screen_w = self.screen_width.max(1.0) as u32;
        let screen_h = self.screen_height.max(1.0) as u32;
        let mut current_pipeline: Option<PipelineKind> = None;
        let mut current_scissor: Option<[u32; 4]> = None;

        for cmd in &self.draws {
            if cmd.index_range.is_empty() {
                continue;
            }
            if current_pipeline != Some(cmd.pipeline_kind) {
                let pipeline = match cmd.pipeline_kind {
                    PipelineKind::Solid => &self.pipelines.solid,
                    PipelineKind::AlphaMask => &self.pipelines.alpha_mask,
                };
                rpass.set_pipeline(pipeline);
                current_pipeline = Some(cmd.pipeline_kind);
            }

            let scissor = cmd.scissor.unwrap_or([0, 0, screen_w, screen_h]);
            if current_scissor != Some(scissor) {
                rpass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
                current_scissor = Some(scissor);
            }

            rpass.set_bind_group(1, self.textures.bind_group(cmd.texture_id), &[]);
            rpass.draw_indexed(cmd.index_range.clone(), 0, 0..1);
        }
    }

    // ---- internal helpers ----

    fn ensure_state(&mut self, texture_id: u32, pipeline: PipelineKind) {
        if texture_id != self.active_texture || pipeline != self.active_pipeline {
            self.commit_batch();
            self.active_texture = texture_id;
            self.active_pipeline = pipeline;
        }
    }

    /// Snapshot the uncommitted index range as one [`DrawCmd`] under the
    /// current texture/pipeline/scissor state.
    fn commit_batch(&mut self) {
        let curr = self.batch.indices.len() as u32;
        if curr <= self.last_index_count {
            return;
        }
        let scissor = self.scissor_stack.last().copied();
        self.draws.push(DrawCmd {
            index_range: self.last_index_count..curr,
            texture_id: self.active_texture,
            pipeline_kind: self.active_pipeline,
            scissor,
        });
        self.last_index_count = curr;
    }

    /// Drain whatever is currently in `glyph_brush`, scheduling atlas
    /// uploads/resizes for `prepare()`, and append the resulting glyph quads
    /// to the batch under an optional scissor.
    fn process_text_batch(&mut self, font_system: &mut FontSystem, scissor: Option<[u32; 4]>) {
        if let Some(s) = scissor {
            self.scissor_stack.push(s);
        }
        self.ensure_state(self.font_texture_id, PipelineKind::AlphaMask);

        const MAX_ITERATIONS: usize = 8;
        let mut iter = 0;

        loop {
            iter += 1;
            if iter > MAX_ITERATIONS {
                eprintln!(
                    "[UiRenderer] glyph_brush process_queued exceeded {} iterations",
                    MAX_ITERATIONS
                );
                break;
            }

            let mut local_uploads: Vec<AtlasUpload> = Vec::new();

            let result = font_system.glyph_brush.process_queued(
                |rect, tex_data| {
                    local_uploads.push(AtlasUpload {
                        x: rect.min[0] as i32,
                        y: rect.min[1] as i32,
                        w: rect.width() as i32,
                        h: rect.height() as i32,
                        data: tex_data.to_vec(),
                    });
                },
                |vertex| {
                    let uv = vertex.tex_coords;
                    let color = vertex.extra.color;
                    let pix = vertex.pixel_coords;
                    UiGlyph {
                        x0: pix.min.x,
                        y0: pix.min.y,
                        x1: pix.max.x,
                        y1: pix.max.y,
                        u0: uv.min.x,
                        v0: uv.min.y,
                        u1: uv.max.x,
                        v1: uv.max.y,
                        color,
                    }
                },
            );

            match result {
                Ok(BrushAction::Draw(verts)) => {
                    self.pending_atlas_uploads.extend(local_uploads);
                    self.cached_glyphs = verts.clone();
                    self.push_glyph_quads(&verts);
                    break;
                }
                Ok(BrushAction::ReDraw) => {
                    // Atlas content unchanged from last frame; ignore any
                    // (empty) upload list and re-emit the cached quads.
                    if !self.cached_glyphs.is_empty() {
                        let cached = self.cached_glyphs.clone();
                        self.push_glyph_quads(&cached);
                    }
                    break;
                }
                Err(BrushError::TextureTooSmall { suggested }) => {
                    let (new_w, new_h) = suggested;
                    font_system.glyph_brush.resize_texture(new_w, new_h);
                    self.pending_atlas_resize = Some((new_w, new_h));
                    // Drop the partial uploads — they targeted the prior
                    // (smaller) atlas which is about to be replaced.
                    self.pending_atlas_uploads.clear();
                    // Loop and re-run process_queued against the new size.
                }
            }
        }

        self.commit_batch();

        if scissor.is_some() {
            self.scissor_stack.pop();
        }
    }

    fn push_glyph_quads(&mut self, glyphs: &[UiGlyph]) {
        for g in glyphs {
            let idx = self.batch.vertices.len() as u32;
            self.batch.vertices.push(UiVertex::new(
                g.x0,
                g.y0,
                g.color,
                [g.u0, g.v0],
                [0.0; 4],
                0.0,
            ));
            self.batch.vertices.push(UiVertex::new(
                g.x1,
                g.y0,
                g.color,
                [g.u1, g.v0],
                [0.0; 4],
                0.0,
            ));
            self.batch.vertices.push(UiVertex::new(
                g.x1,
                g.y1,
                g.color,
                [g.u1, g.v1],
                [0.0; 4],
                0.0,
            ));
            self.batch.vertices.push(UiVertex::new(
                g.x0,
                g.y1,
                g.color,
                [g.u0, g.v1],
                [0.0; 4],
                0.0,
            ));
            self.batch
                .indices
                .extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
        }
    }
}

fn intersect_rects(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let ax2 = a[0].saturating_add(a[2]);
    let ay2 = a[1].saturating_add(a[3]);
    let bx2 = b[0].saturating_add(b[2]);
    let by2 = b[1].saturating_add(b[3]);

    let x = a[0].max(b[0]);
    let y = a[1].max(b[1]);
    let x2 = ax2.min(bx2);
    let y2 = ay2.min(by2);

    [x, y, x2.saturating_sub(x), y2.saturating_sub(y)]
}

fn clip_rect_to_screen(rect: Rect, screen_w: f32, screen_h: f32) -> [u32; 4] {
    let x0 = rect.x.clamp(0.0, screen_w) as u32;
    let y0 = rect.y.clamp(0.0, screen_h) as u32;
    let x1 = (rect.x + rect.width).clamp(0.0, screen_w) as u32;
    let y1 = (rect.y + rect.height).clamp(0.0, screen_h) as u32;
    [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)]
}

fn create_solid_rgba_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: [u8; 4],
    label: &str,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    texture
}

fn create_font_atlas(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ui_font_atlas"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
