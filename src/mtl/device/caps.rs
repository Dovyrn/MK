use super::Device;
use crate::{
    Caps, TextureInfo, atomic64, format, stage, subgroup_op, texture_dimension, texture_usage,
};
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
        Caps {
            subgroup: width,
            subgroup_sizes: width,
            subgroup_ops: subgroup_op::BITS & !subgroup_op::CLUSTERED,
            subgroup_stages: stage::COMPUTE | stage::FRAGMENT,
            full_subgroups: false,
            max_threads: raw.maxThreadsPerThreadgroup().width as u32,
            max_shared: raw.maxThreadgroupMemoryLength() as u32,
            transients: true,
            transient_reads: true,
            attachment_reads: true,
            ordered_reads: true,
            tiles: raw.supportsFamily(MTLGPUFamily::Apple4),
            atomics64: if apple9
                || cfg!(target_os = "macos") && raw.supportsFamily(MTLGPUFamily::Apple8)
            {
                atomic64::MIN_MAX | atomic64::IMAGE
            } else {
                0
            },
            mesh: raw.supportsFamily(MTLGPUFamily::Apple7),
            task: raw.supportsFamily(MTLGPUFamily::Apple7),
            mesh_indirect: raw.supportsFamily(MTLGPUFamily::Apple9),
            indirect_count: true,
            multi_draw: true,
            max_draws: u32::MAX,
            mesh_groups: groups,
            mesh_axis: groups,
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
            viewports: 16,
            graphics: u32::MAX >> (32 - queues),
            anisotropy: 16,
            depth_clamp: true,
            line_fill: true,
            slots: true,
            storage: true,
            persistent: true,
            bind_alignment: 256,
            compute: true,
            texture_views: true,
            occlusion: true,
            bias_clamp: true,
            blends: true,
            indirect: true,
            indirect_offset: true,
            max_queries: 4096,
            cube_arrays: true,
            mirror_clamp: true,
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
            | format::BGR5A1_UNORM => texture_usage::SAMPLED | texture_usage::COLOR,
            format::R8_UNORM
            | format::R8_UINT
            | format::R8_SINT
            | format::R16_FLOAT
            | format::R16_UINT
            | format::R16_SINT
            | format::R32_FLOAT
            | format::RGBA8_UNORM
            | format::RGBA8_UINT
            | format::RGBA8_SINT
            | format::RGBA16_FLOAT
            | format::RGBA16_UINT
            | format::RGBA16_SINT
            | format::RGBA32_FLOAT
            | format::RGBA32_UINT
            | format::RGBA32_SINT => {
                texture_usage::SAMPLED | texture_usage::STORAGE | texture_usage::COLOR
            }
            format::R32_UINT | format::R32_SINT => {
                texture_usage::SAMPLED
                    | texture_usage::STORAGE
                    | texture_usage::ATOMIC
                    | texture_usage::COLOR
            }
            format::A8_UNORM..=format::RGBA32_FLOAT => {
                texture_usage::SAMPLED | texture_usage::COLOR
            }
            format::R64_UINT if self.caps.atomics64 & atomic64::IMAGE != 0 => {
                texture_usage::STORAGE | texture_usage::ATOMIC
            }
            format::D24_UNORM_S8 if !self.raw.isDepth24Stencil8PixelFormatSupported() => 0,
            format::D16_UNORM..=format::S8_UINT => texture_usage::SAMPLED | texture_usage::DEPTH,
            format::BC1_UNORM..=format::BC7_SRGB if !self.raw.supportsBCTextureCompression() => 0,
            format::BC1_UNORM..=format::ASTC_12X12_FLOAT => texture_usage::SAMPLED,
            _ => 0,
        };
        if usage == 0 {
            return 0;
        }
        let usage = if usage & texture_usage::COLOR != 0 {
            usage | texture_usage::INPUT
        } else {
            usage
        };
        let depth = (format::D16_UNORM..=format::S8_UINT).contains(&format);
        let host = if depth || !self.raw.hasUnifiedMemory() {
            0
        } else {
            texture_usage::HOST
        };
        usage | texture_usage::COPY_SRC | texture_usage::COPY_DST | texture_usage::VIEW | host
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let msaa = usage & texture_usage::STORAGE == 0
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
        self.caps.check_texture(info, usable, counts)?;
        let line = matches!(
            info.dimension,
            texture_dimension::D1 | texture_dimension::D1_ARRAY
        );
        let target = info.usage & (texture_usage::COLOR | texture_usage::DEPTH) != 0;
        if cfg!(feature = "validation") && line && target {
            return Err("a 1D texture cannot be a render target on Metal".into());
        }
        Ok(())
    }
}
