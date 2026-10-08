use super::Device;
use crate::Error;
use crate::{
    Atomic64, Caps, Format, Stage, SubgroupOp, TextureDimension, TextureInfo, TextureUsage,
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
            subgroup_ops: !SubgroupOp::CLUSTERED,
            subgroup_stages: Stage::COMPUTE | Stage::FRAGMENT,
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
                Atomic64::MIN_MAX | Atomic64::IMAGE
            } else {
                Atomic64::default()
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

    pub fn format_usage(&self, format: Format) -> TextureUsage {
        let usage = match format {
            Format::B5g6r5Unorm
            | Format::A1bgr5Unorm
            | Format::Abgr4Unorm
            | Format::Bgr5a1Unorm => TextureUsage::SAMPLED | TextureUsage::COLOR,
            Format::R8Unorm
            | Format::R8Uint
            | Format::R8Sint
            | Format::R16Float
            | Format::R16Uint
            | Format::R16Sint
            | Format::R32Float
            | Format::Rgba8Unorm
            | Format::Rgba8Uint
            | Format::Rgba8Sint
            | Format::Rgba16Float
            | Format::Rgba16Uint
            | Format::Rgba16Sint
            | Format::Rgba32Float
            | Format::Rgba32Uint
            | Format::Rgba32Sint => {
                TextureUsage::SAMPLED | TextureUsage::STORAGE | TextureUsage::COLOR
            }
            Format::R32Uint | Format::R32Sint => {
                TextureUsage::SAMPLED
                    | TextureUsage::STORAGE
                    | TextureUsage::ATOMIC
                    | TextureUsage::COLOR
            }
            f if (Format::A8Unorm..=Format::Rgba32Float).contains(&f) => {
                TextureUsage::SAMPLED | TextureUsage::COLOR
            }
            Format::R64Uint if self.caps.atomics64.contains(Atomic64::IMAGE) => {
                TextureUsage::STORAGE | TextureUsage::ATOMIC
            }
            Format::D24UnormS8 if !self.raw.isDepth24Stencil8PixelFormatSupported() => {
                TextureUsage::default()
            }
            f if (Format::D16Unorm..=Format::S8Uint).contains(&f) => {
                TextureUsage::SAMPLED | TextureUsage::DEPTH
            }
            f if (Format::Bc1Unorm..=Format::Bc7Srgb).contains(&f)
                && !self.raw.supportsBCTextureCompression() =>
            {
                TextureUsage::default()
            }
            f if (Format::Bc1Unorm..=Format::Astc12x12Float).contains(&f) => TextureUsage::SAMPLED,
            _ => TextureUsage::default(),
        };
        if usage.is_empty() {
            return usage;
        }
        let usage = if usage.contains(TextureUsage::COLOR) {
            usage | TextureUsage::INPUT
        } else {
            usage
        };
        let depth = (Format::D16Unorm..=Format::S8Uint).contains(&format);
        let host = if depth || !self.raw.hasUnifiedMemory() {
            TextureUsage::default()
        } else {
            TextureUsage::HOST
        };
        usage | TextureUsage::COPY_SRC | TextureUsage::COPY_DST | TextureUsage::VIEW | host
    }

    pub fn format_samples(&self, format: Format, usage: TextureUsage) -> u32 {
        if !self.format_usage(format).contains(usage) {
            return 0;
        }
        let msaa = !usage.contains(TextureUsage::STORAGE)
            && match format {
                f if (Format::Bc1Unorm..=Format::Astc12x12Float).contains(&f) => false,
                Format::R32Uint | Format::R32Sint | Format::Rgba32Uint | Format::Rgba32Sint => {
                    self.raw.supports32BitMSAA()
                }
                Format::R32Float | Format::Rg32Float | Format::Rgba32Float => {
                    self.raw.supports32BitMSAA() && self.raw.supports32BitFloatFiltering()
                }
                _ => true,
            };
        [2, 4, 8]
            .into_iter()
            .filter(|&n| msaa && self.raw.supportsTextureSampleCount(n))
            .fold(1, |m, n| m | n as u32)
    }

    pub(super) fn supports(&self, info: &TextureInfo) -> Result<(), Error> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.check_texture(info, usable, counts)?;
        let line = matches!(
            info.dimension,
            TextureDimension::D1 | TextureDimension::D1Array
        );
        let target = !(info.usage & (TextureUsage::COLOR | TextureUsage::DEPTH)).is_empty();
        if cfg!(feature = "validation") && line && target {
            return Err(Error::Unsupported(
                "a 1D texture cannot be a render target on Metal".into(),
            ));
        }
        Ok(())
    }
}
