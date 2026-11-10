//! Alvo e pipelines dos grafos. Mesma textura para Feixes e Raízes, composta pela cena.

use gfx::wgpu::*;
use gfx::{Gpu, UI_FORMAT};
use graph::{Dot, LineVertex};
use std::mem::size_of;
use std::num::NonZeroU64;

const DRAW_STRIDE: u64 = 256;
pub const MAX_DRAWS: usize = 900;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrameU {
    pub res: [f32; 2],
    pub scale: f32,
    pub light: f32,
    pub span: f32,
    pub head: f32,
    pub offset: [f32; 2],
    pub wscale: f32,
    pub pad: [f32; 3],
    pub pal: [[f32; 4]; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawU {
    pub p: f32,
    pub use_delay: f32,
    pub hi: f32,
    pub hi_a: f32,
    pub add: f32,
    pub rev: f32,
    pub head_gain: f32,
    pub dim: f32,
    pub soft: f32,
    pub pad: [f32; 3],
}

/// Buffer que cresce quando precisa e só escreve o que mudou.
pub struct Grow {
    pub buf: Buffer,
    cap: u64,
    usage: BufferUsages,
}

impl Grow {
    pub fn new(d: &Device, cap: u64, usage: BufferUsages) -> Self {
        let buf = d.create_buffer(&BufferDescriptor { label: Some("graph"), size: cap, usage: usage | BufferUsages::COPY_DST, mapped_at_creation: false });
        Self { buf, cap, usage }
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[u8]) {
        if data.len() as u64 > self.cap {
            *self = Self::new(&gpu.device, (data.len() as u64).next_power_of_two(), self.usage);
        }
        if !data.is_empty() {
            gpu.queue.write_buffer(&self.buf, 0, data);
        }
    }
}

pub struct Target {
    pub view: TextureView,
    pub size: (u32, u32),
}

pub struct Pipes {
    frame: Buffer,
    draws: Buffer,
    bind: BindGroup,
    line_add: RenderPipeline,
    line_sub: RenderPipeline,
    dot_add: RenderPipeline,
    dot_sub: RenderPipeline,
    /// Base: cobre em vez de somar. Cem ligações no mesmo feixe chegam à cor do tema, nunca ao branco.
    line_over: RenderPipeline,
    dot_over: RenderPipeline,
    pub vertices: Grow,
    pub indices: Grow,
    pub dots: Grow,
    pub fx: Grow,
    pub focus: Grow,
    pub glows: Grow,
    /// Arcos soltos: a luz entre leituras e as ligações do foco em Raízes.
    pub arc_vertices: Grow,
    pub arc_indices: Grow,
    /// Poeira e névoa: o fundo do desenho.
    pub backdrop: Grow,
}

/// Uma chamada de desenho, já com seu uniforme na lista.
pub enum Call {
    Base(u32),
    Fx { offset: u32, count: u32 },
    Focus { offset: u32, count: u32 },
    Arc { offset: u32, count: u32 },
    /// A base de novo, larga e suave, por baixo das linhas: o brilho delas.
    BaseGlow(u32),
    /// Poeira e névoa, antes de tudo.
    Backdrop(u32),
    /// Os mesmos pontos da base, maiores e suaves: o halo de cada item.
    Halos(u32),
    Dots(u32),
    Glows(u32),
}

impl Pipes {
    pub fn new(gpu: &Gpu) -> Self {
        let d = &gpu.device;
        let sh = d.create_shader_module(ShaderModuleDescriptor { label: Some("graph"), source: ShaderSource::Wgsl(include_str!("graph.wgsl").into()) });
        let vis = ShaderStages::VERTEX_FRAGMENT;
        let bgl = d.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry { binding: 0, visibility: vis, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 1, visibility: vis, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: NonZeroU64::new(size_of::<DrawU>() as u64) }, count: None },
            ],
        });
        let frame = d.create_buffer(&BufferDescriptor { label: None, size: size_of::<FrameU>() as u64, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false });
        let draws = d.create_buffer(&BufferDescriptor { label: None, size: DRAW_STRIDE * MAX_DRAWS as u64, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = d.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                BindGroupEntry { binding: 0, resource: frame.as_entire_binding() },
                BindGroupEntry { binding: 1, resource: BindingResource::Buffer(BufferBinding { buffer: &draws, offset: 0, size: NonZeroU64::new(size_of::<DrawU>() as u64) }) },
            ],
        });
        let pl = d.create_pipeline_layout(&PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
        let add = BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add };
        let sub = BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::ReverseSubtract };
        let line_attrs = vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32,3=>Float32,4=>Float32,5=>Float32,6=>Float32,7=>Float32x2];
        let dot_attrs = vertex_attr_array![0=>Float32x2,1=>Float32,2=>Float32,3=>Float32,4=>Float32,5=>Float32];
        let over = BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::OneMinusSrcAlpha, operation: BlendOperation::Add };
        let pipe = |vs: &str, fs: &str, attrs: &[VertexAttribute], stride: usize, step: VertexStepMode, color: BlendComponent| {
            d.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&pl),
                vertex: VertexState {
                    module: &sh,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(VertexBufferLayout { array_stride: stride as u64, step_mode: step, attributes: attrs })],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(FragmentState {
                    module: &sh,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState { format: UI_FORMAT, blend: Some(BlendState { color, alpha: BlendComponent::OVER }), write_mask: ColorWrites::COLOR })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let v = BufferUsages::VERTEX;
        let i = BufferUsages::INDEX;
        Self {
            line_add: pipe("line_vs", "line_fs", &line_attrs, size_of::<LineVertex>(), VertexStepMode::Vertex, add),
            line_sub: pipe("line_vs", "line_fs", &line_attrs, size_of::<LineVertex>(), VertexStepMode::Vertex, sub),
            dot_add: pipe("dot_vs", "dot_fs", &dot_attrs, size_of::<Dot>(), VertexStepMode::Instance, add),
            dot_sub: pipe("dot_vs", "dot_fs", &dot_attrs, size_of::<Dot>(), VertexStepMode::Instance, sub),
            line_over: pipe("line_vs", "line_over_fs", &line_attrs, size_of::<LineVertex>(), VertexStepMode::Vertex, over),
            dot_over: pipe("dot_vs", "dot_over_fs", &dot_attrs, size_of::<Dot>(), VertexStepMode::Instance, over),
            frame,
            draws,
            bind,
            vertices: Grow::new(d, 1 << 16, v),
            indices: Grow::new(d, 1 << 16, i),
            dots: Grow::new(d, 1 << 12, v),
            fx: Grow::new(d, 1 << 14, i),
            focus: Grow::new(d, 1 << 14, i),
            glows: Grow::new(d, 1 << 12, v),
            arc_vertices: Grow::new(d, 1 << 14, v),
            arc_indices: Grow::new(d, 1 << 13, i),
            backdrop: Grow::new(d, 1 << 13, v),
        }
    }

    pub fn target(gpu: &Gpu, w: u32, h: u32) -> Target {
        let tex = gpu.device.create_texture(&TextureDescriptor {
            label: Some("graph target"),
            size: Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: UI_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Target { view: tex.create_view(&Default::default()), size: (w, h) }
    }

    pub fn draw(&self, gpu: &Gpu, enc: &mut CommandEncoder, target: &Target, frame: FrameU, clear: [f32; 3], calls: &[(Call, DrawU)]) {
        gpu.queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&frame));
        let mut data = vec![0u8; calls.len() * DRAW_STRIDE as usize];
        for (i, (_, u)) in calls.iter().enumerate() {
            let at = i * DRAW_STRIDE as usize;
            data[at..at + size_of::<DrawU>()].copy_from_slice(bytemuck::bytes_of(u));
        }
        gpu.queue.write_buffer(&self.draws, 0, &data);
        let mut p = enc.begin_render_pass(&RenderPassDescriptor {
            label: Some("graph"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &target.view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(Color { r: clear[0] as f64, g: clear[1] as f64, b: clear[2] as f64, a: 1.0 }), store: StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let light = frame.light > 0.5;
        let (line, dot) = if light { (&self.line_sub, &self.dot_sub) } else { (&self.line_add, &self.dot_add) };
        for (i, (call, _)) in calls.iter().enumerate() {
            p.set_bind_group(0, &self.bind, &[i as u32 * DRAW_STRIDE as u32]);
            match call {
                Call::Base(count) | Call::BaseGlow(count) => {
                    p.set_pipeline(&self.line_over);
                    p.set_vertex_buffer(0, self.vertices.buf.slice(..));
                    p.set_index_buffer(self.indices.buf.slice(..), IndexFormat::Uint32);
                    p.draw_indexed(0..*count, 0, 0..1);
                }
                Call::Fx { offset, count } | Call::Focus { offset, count } => {
                    let buf = if matches!(call, Call::Fx { .. }) { &self.fx } else { &self.focus };
                    p.set_pipeline(line);
                    p.set_vertex_buffer(0, self.vertices.buf.slice(..));
                    p.set_index_buffer(buf.buf.slice(..), IndexFormat::Uint32);
                    p.draw_indexed(*offset..*offset + *count, 0, 0..1);
                }
                Call::Arc { offset, count } => {
                    p.set_pipeline(line);
                    p.set_vertex_buffer(0, self.arc_vertices.buf.slice(..));
                    p.set_index_buffer(self.arc_indices.buf.slice(..), IndexFormat::Uint32);
                    p.draw_indexed(*offset..*offset + *count, 0, 0..1);
                }
                Call::Backdrop(n) => {
                    p.set_pipeline(dot);
                    p.set_vertex_buffer(0, self.backdrop.buf.slice(..));
                    p.draw(0..6, 0..*n);
                }
                Call::Halos(n) => {
                    p.set_pipeline(dot);
                    p.set_vertex_buffer(0, self.dots.buf.slice(..));
                    p.draw(0..6, 0..*n);
                }
                Call::Dots(n) => {
                    p.set_pipeline(&self.dot_over);
                    p.set_vertex_buffer(0, self.dots.buf.slice(..));
                    p.draw(0..6, 0..*n);
                }
                Call::Glows(n) => {
                    p.set_pipeline(dot);
                    p.set_vertex_buffer(0, self.glows.buf.slice(..));
                    p.draw(0..6, 0..*n);
                }
            }
        }
    }
}
