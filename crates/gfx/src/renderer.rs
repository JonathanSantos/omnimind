use crate::gpu::UI_FORMAT;
use crate::scene::{GlyphPrim, Op, RectPrim, Scene, SegPrim};
use crate::text::{ATLAS_SIZE, TextEngine};
use crate::{Color, Gpu};
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use wgpu::*;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Globals {
    pub size: [f32; 2],
    pub scale: f32,
    pub time: f32,
}

struct Grow {
    buf: Buffer,
    cap: u64,
    usage: BufferUsages,
}

impl Grow {
    fn new(d: &Device, cap: u64, usage: BufferUsages) -> Self {
        let buf = d.create_buffer(&BufferDescriptor {
            label: None,
            size: cap,
            usage: usage | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buf, cap, usage }
    }
    fn write(&mut self, gpu: &Gpu, data: &[u8]) {
        if data.len() as u64 > self.cap {
            *self = Self::new(&gpu.device, (data.len() as u64 * 2).next_power_of_two(), self.usage);
        }
        if !data.is_empty() {
            gpu.queue.write_buffer(&self.buf, 0, data);
        }
    }
}

pub struct Renderer {
    globals: Buffer,
    bg0: BindGroup,
    bg1: BindGroup,
    bgl2: BindGroupLayout,
    sampler: Sampler,
    mask: Texture,
    color: Texture,
    rect: RenderPipeline,
    glyph: RenderPipeline,
    seg: RenderPipeline,
    image: RenderPipeline,
    rects: Grow,
    glyphs: Grow,
    segs: Grow,
    images_buf: Grow,
    images: Vec<BindGroup>,
}

fn atlas(d: &Device, format: TextureFormat) -> Texture {
    d.create_texture(&TextureDescriptor {
        label: Some("atlas"),
        size: Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

impl Renderer {
    pub fn new(gpu: &Gpu) -> Self {
        let d = &gpu.device;
        let sh = d.create_shader_module(ShaderModuleDescriptor {
            label: Some("ui"),
            source: ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });
        let vis = ShaderStages::VERTEX_FRAGMENT;
        let bgl0 = d.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: vis,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let tex = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let smp = |binding| BindGroupLayoutEntry {
            binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        };
        let bgl1 = d.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[tex(0), tex(1), smp(2)],
        });
        let bgl2 = d.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[tex(0), smp(1)],
        });
        let globals = d.create_buffer(&BufferDescriptor {
            label: None,
            size: size_of::<Globals>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg0 = d.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bgl0,
            entries: &[BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let mask = atlas(d, TextureFormat::R8Unorm);
        let color = atlas(d, TextureFormat::Rgba8Unorm);
        let nearest = d.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let sampler = d.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        let (mv, cv) = (mask.create_view(&Default::default()), color.create_view(&Default::default()));
        let bg1 = d.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bgl1,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(&mv) },
                BindGroupEntry { binding: 1, resource: BindingResource::TextureView(&cv) },
                BindGroupEntry { binding: 2, resource: BindingResource::Sampler(&nearest) },
            ],
        });
        let pl = d.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl0), Some(&bgl1), Some(&bgl2)],
            immediate_size: 0,
        });
        let premul = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent::OVER,
        };
        let pipe = |vs: &str, fs: &str, attrs: &[VertexAttribute], stride: usize| {
            d.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(vs),
                layout: Some(&pl),
                vertex: VertexState {
                    module: &sh,
                    entry_point: Some(vs),
                    compilation_options: Default::default(),
                    buffers: &[Some(VertexBufferLayout {
                        array_stride: stride as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: attrs,
                    })],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(FragmentState {
                    module: &sh,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: UI_FORMAT,
                        blend: Some(premul),
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let rect = pipe(
            "rect_vs",
            "rect_fs",
            &vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4],
            size_of::<RectPrim>(),
        );
        let glyph = pipe(
            "glyph_vs",
            "glyph_fs",
            &vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32],
            size_of::<GlyphPrim>(),
        );
        let seg = pipe(
            "seg_vs",
            "seg_fs",
            &vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32],
            size_of::<SegPrim>(),
        );
        let image = pipe(
            "image_vs",
            "image_fs",
            &vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4],
            48,
        );
        let v = BufferUsages::VERTEX;
        Self {
            rects: Grow::new(d, 1 << 16, v),
            glyphs: Grow::new(d, 1 << 18, v),
            segs: Grow::new(d, 1 << 14, v),
            images_buf: Grow::new(d, 1 << 10, v),
            globals,
            bg0,
            bg1,
            bgl2,
            sampler,
            mask,
            color,
            rect,
            glyph,
            seg,
            image,
            images: Vec::new(),
        }
    }

    /// Registra uma textura pré-multiplicada para `Scene::image`. Devolve o id.
    pub fn register_image(&mut self, gpu: &Gpu, view: &TextureView, slot: Option<usize>) -> usize {
        let bg = gpu.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.bgl2,
            entries: &[
                BindGroupEntry { binding: 0, resource: BindingResource::TextureView(view) },
                BindGroupEntry { binding: 1, resource: BindingResource::Sampler(&self.sampler) },
            ],
        });
        match slot {
            Some(i) => {
                self.images[i] = bg;
                i
            }
            None => {
                self.images.push(bg);
                self.images.len() - 1
            }
        }
    }

    pub fn upload_text(&mut self, gpu: &Gpu, t: &mut TextEngine) {
        for u in t.uploads.drain(..) {
            let (texture, bpp) = if u.color { (&self.color, 4) } else { (&self.mask, 1) };
            gpu.queue.write_texture(
                TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: Origin3d { x: u.x, y: u.y, z: 0 },
                    aspect: TextureAspect::All,
                },
                &u.data,
                TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(u.w * bpp), rows_per_image: None },
                Extent3d { width: u.w, height: u.h, depth_or_array_layers: 1 },
            );
        }
        t.reset = false;
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        enc: &mut CommandEncoder,
        view: &TextureView,
        scene: &Scene,
        text: &mut TextEngine,
        globals: Globals,
        clear: Color,
    ) {
        self.upload_text(gpu, text);
        gpu.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        self.rects.write(gpu, bytemuck::cast_slice(&scene.rects));
        self.glyphs.write(gpu, bytemuck::cast_slice(&scene.glyphs));
        self.segs.write(gpu, bytemuck::cast_slice(&scene.segs));
        let mut imgs: Vec<[f32; 12]> = Vec::new();
        for op in &scene.ops {
            if let Op::Image { rect, clip, tint, .. } = op {
                let t = tint.premul();
                imgs.push([rect.x, rect.y, rect.w, rect.h, clip.x, clip.y, clip.w, clip.h, t[0], t[1], t[2], t[3]]);
            }
        }
        self.images_buf.write(gpu, bytemuck::cast_slice(&imgs));
        let mut p = enc.begin_render_pass(&RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: clear.r as f64,
                        g: clear.g as f64,
                        b: clear.b as f64,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        p.set_bind_group(0, &self.bg0, &[]);
        p.set_bind_group(1, &self.bg1, &[]);
        if let Some(first) = self.images.first() {
            p.set_bind_group(2, first, &[]);
        } else {
            let empty = self.empty_image(gpu);
            p.set_bind_group(2, &empty, &[]);
        }
        let mut img_i = 0u32;
        for op in &scene.ops {
            match op {
                Op::Rects(r) => {
                    p.set_pipeline(&self.rect);
                    p.set_vertex_buffer(0, self.rects.buf.slice(..));
                    p.draw(0..6, r.clone());
                }
                Op::Glyphs(r) => {
                    p.set_pipeline(&self.glyph);
                    p.set_vertex_buffer(0, self.glyphs.buf.slice(..));
                    p.draw(0..6, r.clone());
                }
                Op::Segs(r) => {
                    p.set_pipeline(&self.seg);
                    p.set_vertex_buffer(0, self.segs.buf.slice(..));
                    p.draw(0..6, r.clone());
                }
                Op::Image { image, .. } => {
                    p.set_pipeline(&self.image);
                    p.set_bind_group(2, &self.images[*image], &[]);
                    p.set_vertex_buffer(0, self.images_buf.buf.slice(..));
                    p.draw(0..6, img_i..img_i + 1);
                    img_i += 1;
                }
            }
        }
    }

    fn empty_image(&mut self, gpu: &Gpu) -> BindGroup {
        let tex = gpu.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let i = self.register_image(gpu, &view, None);
        self.images.remove(i)
    }
}
