use super::Device;
use crate::{Caps, TextureInfo, format, usage};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLGPUFamily};

impl Device {
    pub(super) fn detect(raw: &ProtocolObject<dyn MTLDevice>, queues: usize) -> Caps {
        Caps {
            mesh: raw.supportsFamily(MTLGPUFamily::Apple7)
                || raw.supportsFamily(MTLGPUFamily::Mac2),
            push: 256,
            queues: queues as u32,
            samplers: 2048,
            textures: 500_000,
            tick: 1e9 / raw.queryTimestampFrequency() as f32,
            timestamps: true,
            precise: true,
            ray: raw.supportsRaytracing(),
            generated: false,
            handle_size: 0,
            handle_alignment: 0,
            table_alignment: 0,
            sparse: raw.supportsFamily(MTLGPUFamily::Apple6),
            shading_rate: false,
            host_memory: true,
            host_alignment: 16384,
            shared_memory: false,
            viewports: 16,
            depth_bounds: raw.supportsFamily(MTLGPUFamily::Apple10),
            conditional: false,
            statistics: false,
            instance: 72,
            memoryless: raw.supportsFamily(MTLGPUFamily::Apple2),
            views: (1..=8)
                .rev()
                .find(|&n| raw.supportsVertexAmplificationCount(n))
                .unwrap_or(1) as u32,
            graphics: u32::MAX >> (32 - queues),
            labels: true,
            rays: false,
            anisotropy: true,
            depth_clamp: true,
            line_fill: true,
            multi_draw: true,
            sparse_textures: raw.supportsFamily(MTLGPUFamily::Apple6),
            slots: true,
            storage: true,
            persistent: true,
            bind_alignment: 256,
            buffers: 500_000,
            compute: true,
            texture_views: true,
            occlusion: true,
            cache: true,
            bias_clamp: true,
            blends: true,
            indirect: true,
            indirect_count: false,
            indirect_ids: true,
            indirect_offset: true,
            wide_lines: false,
            max_queries: 4096,
            cube_arrays: true,
            mirror_clamp: true,
            max_buffer: raw.maxBufferLength() as u64,
            max_texture: 16384,
            max_cube: 16384,
            max_volume: 2048,
            max_layers: 2048,
            budget: true,
        }
    }

    pub fn caps(&self) -> Caps {
        self.caps
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        let usage = match format {
            format::B5G6R5_UNORM
            | format::A1BGR5_UNORM
            | format::ABGR4_UNORM
            | format::BGR5A1_UNORM => usage::SAMPLED | usage::COLOR,
            format::A8_UNORM..=format::RGBA32_FLOAT => {
                usage::SAMPLED | usage::STORAGE | usage::COLOR
            }
            format::D24_UNORM_S8 if !self.raw.isDepth24Stencil8PixelFormatSupported() => 0,
            format::D16_UNORM..=format::S8_UINT => usage::SAMPLED | usage::DEPTH,
            format::BC1_UNORM..=format::BC7_SRGB if !self.raw.supportsBCTextureCompression() => 0,
            format::BC1_UNORM..=format::ASTC_12X12_FLOAT => usage::SAMPLED,
            _ => 0,
        };
        if usage == 0 {
            return 0;
        }
        let host = if (format::D16_UNORM..=format::S8_UINT).contains(&format) {
            0
        } else {
            usage::HOST
        };
        usage | usage::COPY_SRC | usage::COPY_DST | usage::VIEW | host
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let msaa = usage & usage::STORAGE == 0
            && match format {
                format::BC1_UNORM..=format::ASTC_12X12_FLOAT => false,
                format::R32_UINT | format::R32_SINT | format::RGBA32_UINT | format::RGBA32_SINT => {
                    self.raw.supports32BitMSAA()
                }
                _ => true,
            };
        [2, 4, 8]
            .into_iter()
            .filter(|&n| msaa && self.raw.supportsTextureSampleCount(n))
            .fold(1, |m, n| m | n as u32)
    }

    pub(super) fn supports(&self, info: &TextureInfo) -> Result<(), String> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)
    }
}
