use super::Device;
use crate::{Caps, TextureInfo, atomic, dimension, format, stage, subgroup, usage};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLGPUFamily};

impl Device {
    pub(super) fn detect(raw: &ProtocolObject<dyn MTLDevice>, queues: usize, width: u32) -> Caps {
        let apple9 = raw.supportsFamily(MTLGPUFamily::Apple9);
        let apple10 = raw.supportsFamily(MTLGPUFamily::Apple10);
        let groups = if apple10 {
            4_194_303
        } else if apple9 {
            1_048_575
        } else {
            1024
        };
        let size = if apple10 { 32768 } else { 16384 };
        let sparse = cfg!(target_os = "macos") || raw.supportsFamily(MTLGPUFamily::Apple8);
        Caps {
            subgroup: width,
            subgroup_sizes: width,
            subgroup_ops: subgroup::BITS & !subgroup::CLUSTERED,
            subgroup_stages: stage::COMPUTE | stage::FRAGMENT,
            full_subgroups: false,
            max_threads: raw.maxThreadsPerThreadgroup().width as u32,
            max_shared: raw.maxThreadgroupMemoryLength() as u32,
            atomics64: if apple9
                || cfg!(target_os = "macos") && raw.supportsFamily(MTLGPUFamily::Apple8)
            {
                atomic::MIN_MAX
            } else {
                0
            },
            mesh: raw.supportsFamily(MTLGPUFamily::Apple7),
            mesh_indirect: raw.supportsFamily(MTLGPUFamily::Apple9),
            mesh_groups: groups,
            mesh_axis: groups,
            push: 256,
            queues: queues as u32,
            samplers: (raw.maxArgumentBufferSamplerCount() as u32).min(if apple9 {
                500_000
            } else {
                996
            }),
            textures: 500_000,
            images: 500_000,
            addresses: true,
            tick: 1e9 / raw.queryTimestampFrequency() as f32,
            timestamps: true,
            precise: true,
            ray: raw.supportsRaytracing() && apple9,
            sparse,
            shading_rate: false,
            rate_images: false,
            viewports: 16,
            graphics: u32::MAX >> (32 - queues),
            labels: true,
            anisotropy: true,
            depth_clamp: true,
            line_fill: true,
            multi_draw: true,
            sparse_textures: sparse,
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
            indirect_count: true,
            indirect_offset: true,
            max_queries: 4096,
            cube_arrays: true,
            mirror_clamp: true,
            copy_formats: true,
            texture_clear: false,
            buffer_clear: true,
            partial_resolve: false,
            max_buffer: raw.maxBufferLength() as u64,
            max_uniform: raw.maxBufferLength() as u64,
            max_storage: raw.maxBufferLength() as u64,
            max_texture: size,
            max_cube: size,
            max_volume: 2048,
            max_layers: 2048,
            budget: true,
            host_read: raw.hasUnifiedMemory(),
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
            format::R8_UNORM
            | format::R8_UINT
            | format::R8_SINT
            | format::R16_FLOAT
            | format::R16_UINT
            | format::R16_SINT
            | format::R32_FLOAT
            | format::R32_UINT
            | format::R32_SINT
            | format::RGBA8_UNORM
            | format::RGBA8_UINT
            | format::RGBA8_SINT
            | format::RGBA16_FLOAT
            | format::RGBA16_UINT
            | format::RGBA16_SINT
            | format::RGBA32_FLOAT
            | format::RGBA32_UINT
            | format::RGBA32_SINT => usage::SAMPLED | usage::STORAGE | usage::COLOR,
            format::A8_UNORM..=format::RGBA32_FLOAT => usage::SAMPLED | usage::COLOR,
            format::D24_UNORM_S8 if !self.raw.isDepth24Stencil8PixelFormatSupported() => 0,
            format::D16_UNORM..=format::S8_UINT => usage::SAMPLED | usage::DEPTH,
            format::BC1_UNORM..=format::BC7_SRGB if !self.raw.supportsBCTextureCompression() => 0,
            format::BC1_UNORM..=format::ASTC_12X12_FLOAT => usage::SAMPLED,
            _ => 0,
        };
        if usage == 0 {
            return 0;
        }
        let float = matches!(
            format,
            format::R32_FLOAT | format::RG32_FLOAT | format::RGBA32_FLOAT
        );
        let blit =
            if usage & usage::COLOR != 0 && (!float || self.raw.supports32BitFloatFiltering()) {
                usage::BLIT
            } else {
                0
            };
        let depth = (format::D16_UNORM..=format::S8_UINT).contains(&format);
        let host = if depth || !self.raw.hasUnifiedMemory() {
            0
        } else {
            usage::HOST
        };
        usage | blit | usage::COPY_SRC | usage::COPY_DST | usage::VIEW | host
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
                format::R32_FLOAT | format::RG32_FLOAT | format::RGBA32_FLOAT => {
                    self.raw.supports32BitMSAA() && self.raw.supports32BitFloatFiltering()
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
        self.caps.texture(info, usable, counts)?;
        let line = matches!(info.dimension, dimension::D1 | dimension::D1_ARRAY);
        if line && info.usage & (usage::COLOR | usage::DEPTH) != 0 {
            return Err("a 1D texture cannot be a render target on Metal".into());
        }
        Ok(())
    }
}
