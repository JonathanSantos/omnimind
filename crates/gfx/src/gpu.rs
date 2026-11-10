use std::sync::Arc;
use wgpu::*;

/// Formato de todo alvo de UI. Não-sRGB de propósito: ver `Color`.
pub const UI_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

pub struct Gpu {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl Gpu {
    pub fn new(surface: Option<&Surface<'_>>, instance: Instance) -> Arc<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        }))
        .expect("no GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            label: Some("gfx"),
            memory_hints: MemoryHints::MemoryUsage,
            ..Default::default()
        }))
        .expect("device");
        Arc::new(Self { instance, adapter, device, queue })
    }

    pub fn instance() -> Instance {
        Instance::new(InstanceDescriptor {
            backends: if cfg!(target_os = "macos") { Backends::METAL } else { Backends::PRIMARY | Backends::GL },
            ..InstanceDescriptor::new_without_display_handle()
        })
    }

    pub fn headless() -> Arc<Self> {
        Self::new(None, Self::instance())
    }
}

/// Alvo sem janela, usado pelo harness para ver e medir telas.
pub struct Offscreen {
    pub texture: Texture,
    pub view: TextureView,
    pub width: u32,
    pub height: u32,
}

impl Offscreen {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Self {
        let texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("offscreen"),
            size: Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: UI_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self { texture, view, width, height }
    }

    /// RGBA8 linha a linha, de cima para baixo.
    pub fn read_rgba(&self, gpu: &Gpu) -> Vec<u8> {
        let stride = (self.width * 4).next_multiple_of(256);
        let buffer = gpu.device.create_buffer(&BufferDescriptor {
            label: Some("readback"),
            size: (stride * self.height) as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: None,
                },
            },
            Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        gpu.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        gpu.device.poll(PollType::wait_indefinitely()).expect("poll");
        let mapped = slice.get_mapped_range().expect("map");
        let mut out = Vec::with_capacity((self.width * self.height * 4) as usize);
        for row in mapped.chunks_exact(stride as usize).take(self.height as usize) {
            for px in row[..self.width as usize * 4].chunks_exact(4) {
                out.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        out
    }

    pub fn save_png(&self, gpu: &Gpu, path: &std::path::Path) -> std::io::Result<()> {
        let rgba = self.read_rgba(gpu);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = std::fs::File::create(path)?;
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);
        enc.set_color(png::ColorType::Rgba);
        let mut w = enc.write_header().map_err(std::io::Error::other)?;
        w.write_image_data(&rgba).map_err(std::io::Error::other)
    }
}
