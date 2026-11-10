//! Pipelines do Universo. Desenha no mesmo alvo dos outros grafos.

use super::gpu::{Grow, Target};
use gfx::wgpu::*;
use gfx::{Gpu, UI_FORMAT};
use std::mem::size_of;
use std::num::NonZeroU64;

const DRAW_STRIDE: u64 = 256;
const MAX_DRAWS: usize = 8;

/// Um corpo, um brilho ou um grão de poeira. `kind`: 0 estrela, 1 planeta, 2 brilho, 3 poeira.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BodyI {
    pub pos: [f32; 3],
    pub size: f32,
    pub theme: f32,
    pub alpha: f32,
    pub delay: f32,
    pub kind: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LinkI {
    pub a: [f32; 3],
    pub theme_a: f32,
    pub b: [f32; 3],
    pub theme_b: f32,
    pub alpha: f32,
    pub width: f32,
    pub delay: f32,
    pub pad: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FrameU {
    pub right: [f32; 4],
    pub up: [f32; 4],
    pub fwd: [f32; 4],
    pub eye: [f32; 4],
    pub res: [f32; 2],
    pub focal: f32,
    pub light: f32,
    pub fog: [f32; 2],
    pub pad: [f32; 2],
    pub pal: [[f32; 4]; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawU {
    pub open: f32,
    pub spread: f32,
    pub dim: f32,
    pub pad: f32,
}

pub enum Call {
    Links(u32),
    Bodies(u32),
    FxLinks(u32),
    FxBodies(u32),
}

pub struct Pipes {
    frame: Buffer,
    draws: Buffer,
    bind: BindGroup,
    body_add: RenderPipeline,
    body_sub: RenderPipeline,
    link_add: RenderPipeline,
    link_sub: RenderPipeline,
    pub bodies: Grow,
    pub links: Grow,
    pub fx_bodies: Grow,
    pub fx_links: Grow,
}

impl Pipes {
    pub fn new(gpu: &Gpu) -> Self {
        let d = &gpu.device;
        let sh = d.create_shader_module(ShaderModuleDescriptor { label: Some("universe"), source: ShaderSource::Wgsl(include_str!("universe.wgsl").into()) });
        let vis = ShaderStages::VERTEX_FRAGMENT;
        let draw_size = NonZeroU64::new(size_of::<DrawU>() as u64);
        let bgl = d.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry { binding: 0, visibility: vis, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                BindGroupLayoutEntry { binding: 1, visibility: vis, ty: BindingType::Buffer { ty: BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: draw_size }, count: None },
            ],
        });
        let frame = d.create_buffer(&BufferDescriptor { label: None, size: size_of::<FrameU>() as u64, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false });
        let draws = d.create_buffer(&BufferDescriptor { label: None, size: DRAW_STRIDE * MAX_DRAWS as u64, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = d.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[BindGroupEntry { binding: 0, resource: frame.as_entire_binding() }, BindGroupEntry { binding: 1, resource: BindingResource::Buffer(BufferBinding { buffer: &draws, offset: 0, size: draw_size }) }],
        });
        let pl = d.create_pipeline_layout(&PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
        let add = BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::Add };
        let sub = BlendComponent { src_factor: BlendFactor::One, dst_factor: BlendFactor::One, operation: BlendOperation::ReverseSubtract };
        let body_attrs = vertex_attr_array![0=>Float32x3,1=>Float32,2=>Float32,3=>Float32,4=>Float32,5=>Float32];
        let link_attrs = vertex_attr_array![0=>Float32x3,1=>Float32,2=>Float32x3,3=>Float32,4=>Float32,5=>Float32,6=>Float32];
        let pipe = |vs: &str, fs: &str, attrs: &[VertexAttribute], stride: usize, color: BlendComponent| {
            d.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&pl),
                vertex: VertexState {
                    module: &sh,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(VertexBufferLayout { array_stride: stride as u64, step_mode: VertexStepMode::Instance, attributes: attrs })],
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
        Self {
            body_add: pipe("body_vs", "body_fs", &body_attrs, size_of::<BodyI>(), add),
            body_sub: pipe("body_vs", "body_fs", &body_attrs, size_of::<BodyI>(), sub),
            link_add: pipe("link_vs", "link_fs", &link_attrs, size_of::<LinkI>(), add),
            link_sub: pipe("link_vs", "link_fs", &link_attrs, size_of::<LinkI>(), sub),
            frame,
            draws,
            bind,
            bodies: Grow::new(d, 1 << 14, v),
            links: Grow::new(d, 1 << 14, v),
            fx_bodies: Grow::new(d, 1 << 12, v),
            fx_links: Grow::new(d, 1 << 13, v),
        }
    }

    pub fn draw(&self, gpu: &Gpu, enc: &mut CommandEncoder, target: &Target, frame: FrameU, clear: [f32; 3], calls: &[(Call, DrawU)]) {
        gpu.queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&frame));
        let calls = &calls[..calls.len().min(MAX_DRAWS)];
        let mut data = vec![0u8; calls.len() * DRAW_STRIDE as usize];
        for (i, (_, u)) in calls.iter().enumerate() {
            let at = i * DRAW_STRIDE as usize;
            data[at..at + size_of::<DrawU>()].copy_from_slice(bytemuck::bytes_of(u));
        }
        gpu.queue.write_buffer(&self.draws, 0, &data);
        let mut p = enc.begin_render_pass(&RenderPassDescriptor {
            label: Some("universe"),
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
        let (body, link) = if light { (&self.body_sub, &self.link_sub) } else { (&self.body_add, &self.link_add) };
        for (i, (call, _)) in calls.iter().enumerate() {
            p.set_bind_group(0, &self.bind, &[i as u32 * DRAW_STRIDE as u32]);
            let (pipe, buf, n) = match call {
                Call::Links(n) => (link, &self.links, *n),
                Call::Bodies(n) => (body, &self.bodies, *n),
                Call::FxLinks(n) => (link, &self.fx_links, *n),
                Call::FxBodies(n) => (body, &self.fx_bodies, *n),
            };
            if n == 0 {
                continue;
            }
            p.set_pipeline(pipe);
            p.set_vertex_buffer(0, buf.buf.slice(..));
            p.draw(0..6, 0..n);
        }
    }
}
