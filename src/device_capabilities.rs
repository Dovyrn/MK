use crate::Error;
use crate::{
    AddressMode, BufferUsage, ColorTarget, DeviceCapabilities, DynamicPipelineStates, FillMode,
    Format, Memory, MeshPipelineInfo, QueryKind, RenderPipelineInfo, SamplerInfo, TextureDimension,
    TextureInfo, TextureUsage,
};

fn need(name: &str, has: bool, cap: &str) -> Result<(), Error> {
    if has || !cfg!(feature = "validation") {
        Ok(())
    } else {
        Err(Error::Unsupported(format!("{name} needs caps.{cap}")))
    }
}

fn assert_format(name: &str, field: &str, f: Format) {
    check_ne!(f, Format::None, "{name} {field} is Format::None");
}

fn assert_samples(name: &str, n: u32) {
    if !cfg!(feature = "validation") {
        return;
    }
    check!(
        n.is_power_of_two() && n <= 64,
        "{name} samples {n} is not a power of two up to 64"
    );
}

pub(crate) fn is_compressed(f: Format) -> bool {
    let [_, w, h] = f.block();
    w > 1 || h > 1
}

pub(crate) fn is_depth_format(f: Format) -> bool {
    (Format::D16Unorm..=Format::S8Uint).contains(&f)
}

pub(crate) fn same_blend(colors: &[ColorTarget]) -> bool {
    let key = |c: &ColorTarget| {
        (
            c.src_color,
            c.dst_color,
            c.op_color,
            c.src_alpha,
            c.dst_alpha,
            c.op_alpha,
        )
    };
    let mut on = colors.iter().filter(|c| c.blend).map(key);
    let first = on.next();
    on.all(|k| Some(k) == first)
}

pub(crate) fn check_support(
    name: &str,
    format: Format,
    usage: TextureUsage,
    samples: u32,
    usable: TextureUsage,
    counts: u32,
) -> Result<(), Error> {
    if !cfg!(feature = "validation") {
        return Ok(());
    }
    if usable.is_empty() {
        return Err(Error::Unsupported(format!(
            "{name} format {format} is not supported; format_usage returns 0 for it"
        )));
    }
    let missing = usage & !usable;
    if !missing.is_empty() {
        return Err(Error::Unsupported(format!(
            "{name} format {format} lacks usage {missing:#x}; check format_usage first"
        )));
    }
    if counts & samples == 0 {
        return Err(Error::Unsupported(format!(
            "{name} format {format} has no {samples}-sample support with usage {usage:#x}; check format_samples first"
        )));
    }
    Ok(())
}

pub(crate) fn check_targets(
    name: &str,
    colors: &[ColorTarget],
    depth: Option<Format>,
    stencil: Option<Format>,
    count: u32,
) {
    if !cfg!(feature = "validation") {
        return;
    }
    for c in colors {
        assert_format(name, "color", c.format);
    }
    for (field, f) in [("depth", depth), ("stencil", stencil)] {
        if let Some(f) = f {
            assert_format(name, field, f);
        }
    }
    assert_samples(name, count);
}

pub(crate) fn assert_not_transient(memory: Memory) {
    if !cfg!(feature = "validation") {
        return;
    }
    check_ne!(
        memory,
        Memory::Transient,
        "create_buffer memory is Memory::Transient, which holds attachments only"
    );
}

pub(crate) fn check_tile_pipeline(info: &crate::TilePipelineInfo) {
    if !cfg!(feature = "validation") {
        return;
    }
    check_targets(
        "create_tile_pipeline",
        info.color_targets,
        None,
        None,
        info.sample_count,
    );
}

impl DeviceCapabilities {
    pub(crate) fn check_render_pipeline(&self, info: &RenderPipelineInfo) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_render_pipeline";
        check_targets(
            name,
            info.color_targets,
            info.depth_format,
            info.stencil_format,
            info.sample_count,
        );
        check!(
            info.vertex_buffers.iter().all(|b| b.stride > 0),
            "{name} vertex buffer stride 0 means a different thing on each API; give a stride"
        );
        need(
            name,
            info.states.depth_bias_clamp == 0.0 || self.bias_clamp,
            "bias_clamp",
        )?;
        self.check_raster(
            name,
            info.color_targets,
            info.fill_mode,
            info.depth_clamp,
            info.ordered_reads,
        )
    }

    pub(crate) fn check_mesh_pipeline(&self, info: &MeshPipelineInfo) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_mesh_pipeline";
        check_targets(
            name,
            info.color_targets,
            info.depth_format,
            info.stencil_format,
            info.sample_count,
        );
        need(name, self.mesh_shaders, "mesh_shaders")?;
        need(
            name,
            info.object_entrypoint.is_empty() || self.object_shaders,
            "object_shaders",
        )?;
        check!(
            !info
                .states
                .dynamic_states
                .contains(DynamicPipelineStates::PRIMITIVE_TOPOLOGY),
            "{name} lists DynamicPipelineStates::PRIMITIVE_TOPOLOGY in MeshPipelineStates.dynamic, but mesh pipelines have no primitive topology"
        );
        need(
            name,
            info.states.depth_bias_clamp == 0.0 || self.bias_clamp,
            "bias_clamp",
        )?;
        self.check_raster(
            name,
            info.color_targets,
            info.fill_mode,
            info.depth_clamp,
            info.ordered_reads,
        )
    }

    fn check_raster(
        &self,
        name: &str,
        colors: &[ColorTarget],
        fill: FillMode,
        clamp: bool,
        ordered: bool,
    ) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        need(name, self.dynamic_rendering, "dynamic_rendering")?;
        need(name, !clamp || self.depth_clamp, "depth_clamp")?;
        need(name, fill != FillMode::Line || self.line_fill, "line_fill")?;
        need(name, !ordered || self.ordered_reads, "ordered_reads")?;
        need(
            name,
            same_blend(colors) || self.independent_blend,
            "independent_blend",
        )
    }

    pub(crate) fn check_indirect(&self, mesh: bool, shader: bool, draws: u32) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_indirect_buffer";
        need(name, !shader || self.buffer_addresses, "buffer_addresses")?;
        need(name, !mesh || self.mesh_indirect, "mesh_indirect")?;
        if draws > self.max_draws {
            return Err(Error::Unsupported(format!(
                "{name} draws {draws} is over caps.max_draws {}",
                self.max_draws
            )));
        }
        Ok(())
    }

    pub(crate) fn check_compute_pipeline(
        &self,
        info: &crate::ComputePipelineInfo,
    ) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_compute_pipeline";
        need(name, self.compute_shaders, "compute_shaders")?;
        let n = info.required_subgroup_size;
        check!(
            n == 0 || n.is_power_of_two(),
            "{name} subgroup {n} is not a power of two"
        );
        need(
            name,
            n == 0 || self.subgroup_size_mask & n != 0,
            "subgroup_size_mask",
        )?;
        need(
            name,
            !info.require_full_subgroups || self.full_subgroups,
            "full_subgroups",
        )
    }

    #[cfg(feature = "vk")]
    pub(crate) fn check_subgroups(
        &self,
        ops: crate::SubgroupOp,
        stage: crate::PipelineStages,
    ) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let missing = ops & !self.subgroup_ops;
        if !missing.is_empty() {
            return Err(Error::Unsupported(format!(
                "the shader uses subgroup ops {missing:#x}, outside caps.subgroup_ops"
            )));
        }
        if !ops.is_empty() && !self.subgroup_stages.contains(stage) {
            return Err(Error::Unsupported(format!(
                "the shader uses subgroup ops in stage {stage:#x}, outside caps.subgroup_stages"
            )));
        }
        Ok(())
    }

    #[cfg(feature = "vk")]
    pub(crate) fn check_atomics(&self, needed: crate::Atomic64) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let missing = needed & !self.atomics64;
        if !missing.is_empty() {
            return Err(Error::Unsupported(format!(
                "the shader uses 64-bit atomics {missing:#x} outside caps.atomics64"
            )));
        }
        Ok(())
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn assert_full_subgroups(&self, width: u32, size: u32) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(
            width.is_multiple_of(size),
            "create_compute_pipeline full needs a thread-group width that is a multiple of {size}, not {width}"
        );
    }

    pub(crate) fn check_buffer(
        &self,
        size: u64,
        memory: Memory,
        usage: BufferUsage,
    ) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_buffer";
        assert_not_transient(memory);
        check!(size > 0, "{name} size is 0");
        check!(!usage.is_empty(), "{name} usage is empty");
        need(name, size <= self.max_buffer, "max_buffer")?;
        let address = usage.contains(BufferUsage::ADDRESS);
        need(name, !address || self.buffer_addresses, "buffer_addresses")
    }

    pub(crate) fn check_timelines(&self, name: &str) -> Result<(), Error> {
        need(name, self.timelines, "timelines")
    }

    pub(crate) fn assert_addresses(&self) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(
            self.buffer_addresses,
            "buffer_address needs caps.buffer_addresses"
        );
    }

    pub(crate) fn assert_image_slot(&self, texture: crate::Texture) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(
            crate::handle::Handle::index(texture) < self.max_storage_textures,
            "bindless_storage_texture_id of {texture} is outside caps.max_storage_textures {}",
            self.max_storage_textures
        );
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn assert_texture_slot(&self, texture: crate::Texture) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(
            crate::handle::Handle::index(texture) < self.max_textures,
            "bindless_texture_id of {texture} is outside caps.max_textures {}",
            self.max_textures
        );
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn assert_sampler_slot(&self, sampler: crate::Sampler) {
        if !cfg!(feature = "validation") {
            return;
        }
        check!(
            crate::handle::Handle::index(sampler) < self.max_samplers,
            "bindless_sampler_id of {sampler} is outside caps.max_samplers {}",
            self.max_samplers
        );
    }

    pub(crate) fn check_texture(
        &self,
        info: &TextureInfo,
        usable: TextureUsage,
        counts: u32,
    ) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_texture";
        let sizes = [
            info.width,
            info.height,
            info.depth,
            info.layer_count,
            info.mip_count,
        ];
        check!(
            sizes.iter().all(|&n| n > 0),
            "{name} width, height, depth, layer_count and mip_count are {sizes:?}; none may be 0"
        );
        assert_format(name, "format", info.format);
        check!(
            !(info.usage & !TextureUsage::MUTABLE_VIEW_FORMAT).is_empty(),
            "{name} usage {:#x} needs more than TextureUsage::MUTABLE_VIEW_FORMAT",
            info.usage
        );
        check!(
            !info.usage.contains(TextureUsage::STORAGE_ATOMIC)
                || info.usage.contains(TextureUsage::STORAGE),
            "{name} usage TextureUsage::STORAGE_ATOMIC needs TextureUsage::STORAGE"
        );
        check!(
            !info.usage.contains(TextureUsage::INPUT_ATTACHMENT)
                || info.usage.contains(TextureUsage::COLOR_ATTACHMENT),
            "{name} usage TextureUsage::INPUT_ATTACHMENT needs TextureUsage::COLOR_ATTACHMENT"
        );
        if info.memory == Memory::Transient {
            let attachment = TextureUsage::COLOR_ATTACHMENT
                | TextureUsage::DEPTH_STENCIL
                | TextureUsage::INPUT_ATTACHMENT;
            check!(
                attachment.contains(info.usage)
                    && !(info.usage
                        & (TextureUsage::COLOR_ATTACHMENT | TextureUsage::DEPTH_STENCIL))
                        .is_empty(),
                "{name} Memory::Transient holds attachments only, not usage {:#x}",
                info.usage
            );
            need(name, self.transient_textures, "transient_textures")?;
            let input = info.usage.contains(TextureUsage::INPUT_ATTACHMENT);
            need(
                name,
                !input || self.transient_input_attachments,
                "transient_input_attachments",
            )?;
        }
        let input = info.usage.contains(TextureUsage::INPUT_ATTACHMENT);
        need(name, !input || self.attachment_reads, "attachment_reads")?;
        assert_samples(name, info.sample_count);
        let largest = info.width.max(info.height).max(info.depth);
        check!(
            info.mip_count <= 32 - largest.leading_zeros(),
            "{name} mip_count {} is more than a {largest} texture has",
            info.mip_count
        );
        let (w, h, d, l) = (info.width, info.height, info.depth, info.layer_count);
        match info.dimension {
            TextureDimension::D1 | TextureDimension::D1Array => check!(
                h == 1 && d == 1 && info.mip_count == 1,
                "{name} 1D texture needs height 1, depth 1 and one mip"
            ),
            TextureDimension::D3 => {
                check_eq!(l, 1, "{name} 3D texture needs one layer");
                check!(
                    (info.usage
                        & (TextureUsage::COLOR_ATTACHMENT | TextureUsage::INPUT_ATTACHMENT))
                        .is_empty(),
                    "{name} 3D texture cannot be an attachment"
                );
            }
            TextureDimension::Cube | TextureDimension::CubeArray => check!(
                w == h && d == 1 && l.is_multiple_of(6),
                "{name} cube texture needs square faces, depth 1 and a multiple of 6 layers"
            ),
            _ => check_eq!(d, 1, "{name} 2D texture needs depth 1"),
        }
        if matches!(
            info.dimension,
            TextureDimension::D1 | TextureDimension::D2 | TextureDimension::D3
        ) {
            check_eq!(l, 1, "{name} needs one layer for a non-array dimension");
        }
        if info.dimension == TextureDimension::Cube {
            check_eq!(l, 6, "{name} cube texture needs 6 layers");
        }
        check!(
            !info.usage.contains(TextureUsage::HOST_COPY) || info.sample_count == 1,
            "{name} TextureUsage::HOST_COPY texture needs one sample"
        );
        if info.sample_count > 1 {
            check!(
                matches!(
                    info.dimension,
                    TextureDimension::D2 | TextureDimension::D2Array
                ) && info.mip_count == 1,
                "{name} multisampled texture needs 2D and one mip"
            );
        }
        if is_compressed(info.format) || is_depth_format(info.format) {
            check!(
                !matches!(
                    info.dimension,
                    TextureDimension::D1 | TextureDimension::D1Array | TextureDimension::D3
                ),
                "{name} format {} is not allowed in a 1D or 3D texture",
                info.format
            );
        }
        let cube = info.dimension == TextureDimension::CubeArray;
        need(name, !cube || self.cube_arrays, "cube_arrays")?;
        let side = w.max(h);
        let (size, limit) = match info.dimension {
            TextureDimension::D3 => (largest, self.max_volume_size),
            TextureDimension::Cube | TextureDimension::CubeArray => (side, self.max_cube_size),
            _ => (side, self.max_texture_size),
        };
        if size > limit {
            return Err(Error::Unsupported(format!(
                "{name} size {size} is over the limit {limit}"
            )));
        }
        need(name, l <= self.max_texture_layers, "max_texture_layers")?;
        check_support(
            name,
            info.format,
            info.usage,
            info.sample_count,
            usable,
            counts,
        )
    }

    pub(crate) fn check_view(&self, kind: TextureDimension) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let cube = kind == TextureDimension::CubeArray;
        need(
            "create_texture_view",
            !cube || self.cube_arrays,
            "cube_arrays",
        )
    }

    pub(crate) fn check_sampler(&self, info: &SamplerInfo) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_sampler";
        check!(
            info.max_anisotropy > 0,
            "{name} max_anisotropy is 0; use 1 for none"
        );
        need(
            name,
            info.max_anisotropy <= self.max_anisotropy_level,
            "max_anisotropy_level",
        )?;
        let mirror = [info.u, info.v, info.w].contains(&AddressMode::MirrorClamp);
        need(name, !mirror || self.mirror_clamp, "mirror_clamp")
    }

    pub(crate) fn check_query_pool(&self, kind: QueryKind, count: u32) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let name = "create_query_pool";
        check!(count > 0, "{name} count is 0");
        need(name, count <= self.max_queries, "max_queries")?;
        need(
            name,
            kind != QueryKind::Timestamp || self.timestamps,
            "timestamps",
        )?;
        need(
            name,
            kind != QueryKind::OcclusionCount || self.precise_occlusion,
            "precise_occlusion",
        )?;
        need(
            name,
            kind != QueryKind::OcclusionAny || self.occlusion_queries,
            "occlusion",
        )
    }
}
