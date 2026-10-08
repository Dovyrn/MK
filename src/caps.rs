use crate::check::assert_known;
use crate::{
    Caps, ColorTarget, DepthStateInfo, MeshPipelineInfo, RenderPipelineInfo, SamplerInfo,
    StencilFace, TextureInfo, address_mode, blend_factor, blend_op, border_color, color_mask,
    compare_op, fill_mode, filter, format, memory, query_kind, stencil_op, texture_dimension,
    texture_usage, topology,
};

fn need(name: &str, has: bool, cap: &str) -> Result<(), String> {
    if has {
        Ok(())
    } else {
        Err(format!("{name} needs caps.{cap}"))
    }
}

fn is_real_format(f: u32) -> bool {
    f > 0 && format::block(f)[0] > 0
}

fn assert_format(name: &str, field: &str, f: u32) {
    assert!(
        is_real_format(f),
        "{name} {field} {f} is no format constant"
    );
}

fn assert_samples(name: &str, n: u32) {
    assert!(
        n.is_power_of_two() && n <= 64,
        "{name} samples {n} is not a power of two up to 64"
    );
}

pub(crate) fn is_compressed(f: u32) -> bool {
    let [_, w, h] = format::block(f);
    w > 1 || h > 1
}

pub(crate) fn is_depth_format(f: u32) -> bool {
    (format::D16_UNORM..=format::S8_UINT).contains(&f)
}

pub(crate) fn same_blend(colors: &[ColorTarget]) -> bool {
    let key = |c: &ColorTarget| (c.src, c.dst, c.op, c.src_alpha, c.dst_alpha, c.op_alpha);
    let mut on = colors.iter().filter(|c| c.blend).map(key);
    let first = on.next();
    on.all(|k| Some(k) == first)
}

pub(crate) fn check_support(
    name: &str,
    format: u32,
    usage: u32,
    samples: u32,
    usable: u32,
    counts: u32,
) -> Result<(), String> {
    if usable == 0 {
        return Err(format!(
            "{name} format {format} is not supported; format_usage returns 0 for it"
        ));
    }
    let missing = usage & !usable;
    if missing != 0 {
        return Err(format!(
            "{name} format {format} lacks usage {missing:#x}; check format_usage first"
        ));
    }
    if counts & samples == 0 {
        return Err(format!(
            "{name} format {format} has no {samples}-sample support with usage {usage:#x}; check format_samples first"
        ));
    }
    Ok(())
}

pub(crate) fn check_targets(
    name: &str,
    colors: &[ColorTarget],
    depth: u32,
    stencil: u32,
    count: u32,
    mode: u32,
) {
    for c in colors {
        assert_format(name, "color", c.format);
        for b in [c.src, c.dst, c.src_alpha, c.dst_alpha] {
            assert_known(
                name,
                "blend factor",
                blend_factor::name(b),
                blend_factor::valid(b),
            );
        }
        for o in [c.op, c.op_alpha] {
            assert_known(name, "blend op", blend_op::name(o), blend_op::valid(o));
        }
        assert_eq!(
            c.mask & !color_mask::BITS,
            0,
            "{name} color mask has bits {:#x} that are no color_mask constant",
            c.mask & !color_mask::BITS
        );
    }
    for (field, f) in [("depth", depth), ("stencil", stencil)] {
        if f != format::NONE {
            assert_format(name, field, f);
        }
    }
    assert_samples(name, count);
    assert_known(name, "fill", fill_mode::name(mode), fill_mode::valid(mode));
}

pub(crate) fn assert_not_transient(memory: u32) {
    assert_ne!(
        memory,
        memory::TRANSIENT,
        "create_buffer memory is memory::TRANSIENT, which holds attachments only"
    );
}

pub(crate) fn check_tile_pipeline(info: &crate::TilePipelineInfo) {
    check_targets(
        "create_tile_pipeline",
        info.colors,
        format::NONE,
        format::NONE,
        info.samples,
        fill_mode::FILL,
    );
}

fn check_stencil_face(f: &StencilFace) {
    let name = "create_depth_state";
    for o in [f.fail, f.depth_fail, f.pass] {
        assert_known(
            name,
            "stencil op",
            stencil_op::name(o),
            stencil_op::valid(o),
        );
    }
    assert_known(
        name,
        "compare",
        compare_op::name(f.compare),
        compare_op::valid(f.compare),
    );
}

impl Caps {
    pub(crate) fn check_render_pipeline(&self, info: &RenderPipelineInfo) -> Result<(), String> {
        let name = "create_render_pipeline";
        check_targets(
            name,
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        let t = info.topology;
        assert_known(name, "topology", topology::name(t), topology::valid(t));
        self.check_raster(name, info.colors, info.fill, info.clamp, info.ordered)
    }

    pub(crate) fn check_mesh_pipeline(&self, info: &MeshPipelineInfo) -> Result<(), String> {
        let name = "create_mesh_pipeline";
        check_targets(
            name,
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        need(name, self.mesh, "mesh")?;
        need(name, info.task.is_empty() || self.task, "task")?;
        self.check_raster(name, info.colors, info.fill, info.clamp, info.ordered)
    }

    fn check_raster(
        &self,
        name: &str,
        colors: &[ColorTarget],
        fill: u32,
        clamp: bool,
        ordered: bool,
    ) -> Result<(), String> {
        need(name, !clamp || self.depth_clamp, "depth_clamp")?;
        need(name, fill != fill_mode::LINE || self.line_fill, "line_fill")?;
        need(name, !ordered || self.ordered_reads, "ordered_reads")?;
        need(name, same_blend(colors) || self.blends, "blends")
    }

    pub(crate) fn check_indirect(&self, mesh: bool, draws: u32) -> Result<(), String> {
        let name = "create_indirect_buffer";
        need(name, self.addresses, "addresses")?;
        need(name, !mesh || self.mesh_indirect, "mesh_indirect")?;
        if draws > self.max_draws {
            return Err(format!(
                "{name} draws {draws} is over caps.max_draws {}",
                self.max_draws
            ));
        }
        Ok(())
    }

    pub(crate) fn check_compute_pipeline(
        &self,
        info: &crate::ComputePipelineInfo,
    ) -> Result<(), String> {
        let name = "create_compute_pipeline";
        need(name, self.compute, "compute")?;
        let n = info.subgroup;
        assert!(
            n == 0 || n.is_power_of_two(),
            "{name} subgroup {n} is not a power of two"
        );
        need(
            name,
            n == 0 || self.subgroup_sizes & n != 0,
            "subgroup_sizes",
        )?;
        need(name, !info.full || self.full_subgroups, "full_subgroups")
    }

    #[cfg(feature = "vk")]
    pub(crate) fn check_subgroups(&self, ops: u32, stage: u32) -> Result<(), String> {
        let missing = ops & !self.subgroup_ops;
        if missing != 0 {
            return Err(format!(
                "the shader uses subgroup ops {missing:#x}, outside caps.subgroup_ops"
            ));
        }
        if ops != 0 && self.subgroup_stages & stage == 0 {
            return Err(format!(
                "the shader uses subgroup ops in stage {stage:#x}, outside caps.subgroup_stages"
            ));
        }
        Ok(())
    }

    #[cfg(feature = "vk")]
    pub(crate) fn check_atomics(&self, needed: u32) -> Result<(), String> {
        let missing = needed & !self.atomics64;
        if missing != 0 {
            return Err(format!(
                "the shader uses 64-bit atomics {missing:#x} outside caps.atomics64"
            ));
        }
        Ok(())
    }

    #[cfg(feature = "vk")]
    pub(crate) fn assert_full_subgroups(&self, width: u32, size: u32) {
        assert!(
            width.is_multiple_of(size),
            "create_compute_pipeline full needs a thread-group width that is a multiple of {size}, not {width}"
        );
    }

    pub(crate) fn check_buffer(&self, size: u64, memory: u32) -> Result<(), String> {
        let name = "create_buffer";
        assert_known(name, "memory", memory::name(memory), memory::valid(memory));
        assert_not_transient(memory);
        assert!(size > 0, "{name} size is 0");
        need(name, size <= self.max_buffer, "max_buffer")
    }

    pub(crate) fn assert_addresses(&self) {
        assert!(self.addresses, "buffer_address needs caps.addresses");
    }

    pub(crate) fn assert_image_slot(&self, texture: crate::Texture) {
        assert!(
            crate::handle::Handle::index(texture) < self.images,
            "bindless_storage_texture of {texture} is outside caps.images {}",
            self.images
        );
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn assert_texture_slot(&self, texture: crate::Texture) {
        assert!(
            crate::handle::Handle::index(texture) < self.textures,
            "bindless_texture of {texture} is outside caps.textures {}",
            self.textures
        );
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn assert_sampler_slot(&self, sampler: crate::Sampler) {
        assert!(
            crate::handle::Handle::index(sampler) < self.samplers,
            "bindless_sampler of {sampler} is outside caps.samplers {}",
            self.samplers
        );
    }

    pub(crate) fn check_texture(
        &self,
        info: &TextureInfo,
        usable: u32,
        counts: u32,
    ) -> Result<(), String> {
        let name = "create_texture";
        let sizes = [info.width, info.height, info.depth, info.layers, info.mips];
        assert!(
            sizes.iter().all(|&n| n > 0),
            "{name} width, height, depth, layers and mips are {sizes:?}; none may be 0"
        );
        let d = info.dimension;
        assert_known(
            name,
            "dimension",
            texture_dimension::name(d),
            texture_dimension::valid(d),
        );
        let m = info.memory;
        assert_known(name, "memory", memory::name(m), memory::valid(m));
        assert_format(name, "format", info.format);
        assert_ne!(
            info.usage & !texture_usage::VIEW,
            0,
            "{name} usage {:#x} needs more than texture_usage::VIEW",
            info.usage
        );
        assert_eq!(
            info.usage & !texture_usage::BITS,
            0,
            "{name} usage has bits {:#x} that are no texture_usage constant",
            info.usage & !texture_usage::BITS
        );
        assert!(
            info.usage & texture_usage::ATOMIC == 0 || info.usage & texture_usage::STORAGE != 0,
            "{name} usage texture_usage::ATOMIC needs texture_usage::STORAGE"
        );
        assert!(
            info.usage & texture_usage::INPUT == 0 || info.usage & texture_usage::COLOR != 0,
            "{name} usage texture_usage::INPUT needs texture_usage::COLOR"
        );
        if info.memory == memory::TRANSIENT {
            let attachment = texture_usage::COLOR | texture_usage::DEPTH | texture_usage::INPUT;
            assert!(
                info.usage & !attachment == 0
                    && info.usage & (texture_usage::COLOR | texture_usage::DEPTH) != 0,
                "{name} memory::TRANSIENT holds attachments only, not usage {:#x}",
                info.usage
            );
            need(name, self.transients, "transients")?;
            let input = info.usage & texture_usage::INPUT != 0;
            need(name, !input || self.transient_reads, "transient_reads")?;
        }
        let input = info.usage & texture_usage::INPUT != 0;
        need(name, !input || self.attachment_reads, "attachment_reads")?;
        assert_samples(name, info.samples);
        let largest = info.width.max(info.height).max(info.depth);
        assert!(
            info.mips <= 32 - largest.leading_zeros(),
            "{name} mips {} is more than a {largest} texture has",
            info.mips
        );
        let (w, h, d, l) = (info.width, info.height, info.depth, info.layers);
        match info.dimension {
            texture_dimension::D1 | texture_dimension::D1_ARRAY => assert!(
                h == 1 && d == 1 && info.mips == 1,
                "{name} 1D texture needs height 1, depth 1 and one mip"
            ),
            texture_dimension::D3 => assert_eq!(l, 1, "{name} 3D texture needs one layer"),
            texture_dimension::CUBE | texture_dimension::CUBE_ARRAY => assert!(
                w == h && d == 1 && l.is_multiple_of(6),
                "{name} cube texture needs square faces, depth 1 and a multiple of 6 layers"
            ),
            _ => assert_eq!(d, 1, "{name} 2D texture needs depth 1"),
        }
        if matches!(
            info.dimension,
            texture_dimension::D1 | texture_dimension::D2 | texture_dimension::D3
        ) {
            assert_eq!(l, 1, "{name} needs one layer for a non-array dimension");
        }
        if info.dimension == texture_dimension::CUBE {
            assert_eq!(l, 6, "{name} cube texture needs 6 layers");
        }
        assert!(
            info.usage & texture_usage::HOST == 0 || info.samples == 1,
            "{name} texture_usage::HOST texture needs one sample"
        );
        if info.samples > 1 {
            assert!(
                matches!(
                    info.dimension,
                    texture_dimension::D2 | texture_dimension::D2_ARRAY
                ) && info.mips == 1,
                "{name} multisampled texture needs 2D and one mip"
            );
        }
        if is_compressed(info.format) || is_depth_format(info.format) {
            assert!(
                !matches!(
                    info.dimension,
                    texture_dimension::D1 | texture_dimension::D1_ARRAY | texture_dimension::D3
                ),
                "{name} format {} is not allowed in a 1D or 3D texture",
                info.format
            );
        }
        let cube = info.dimension == texture_dimension::CUBE_ARRAY;
        need(name, !cube || self.cube_arrays, "cube_arrays")?;
        let side = w.max(h);
        let (size, limit) = match info.dimension {
            texture_dimension::D3 => (largest, self.max_volume),
            texture_dimension::CUBE | texture_dimension::CUBE_ARRAY => (side, self.max_cube),
            _ => (side, self.max_texture),
        };
        if size > limit {
            return Err(format!("{name} size {size} is over the limit {limit}"));
        }
        need(name, l <= self.max_layers, "max_layers")?;
        check_support(name, info.format, info.usage, info.samples, usable, counts)
    }

    pub(crate) fn check_view(&self, kind: u32) -> Result<(), String> {
        let cube = kind == texture_dimension::CUBE_ARRAY;
        need(
            "create_texture_view",
            !cube || self.cube_arrays,
            "cube_arrays",
        )
    }

    pub(crate) fn check_sampler(&self, info: &SamplerInfo) -> Result<(), String> {
        let name = "create_sampler";
        for f in [info.min, info.mag, info.mip] {
            assert_known(name, "filter", filter::name(f), filter::valid(f));
        }
        for a in [info.u, info.v, info.w] {
            assert_known(
                name,
                "address",
                address_mode::name(a),
                address_mode::valid(a),
            );
        }
        let b = info.border;
        assert_known(
            name,
            "border",
            border_color::name(b),
            border_color::valid(b),
        );
        let c = info.compare;
        assert_known(name, "compare", compare_op::name(c), compare_op::valid(c));
        assert!(
            info.anisotropy > 0,
            "{name} anisotropy is 0; use 1 for none"
        );
        need(name, info.anisotropy <= self.anisotropy, "anisotropy")?;
        let mirror = [info.u, info.v, info.w].contains(&address_mode::MIRROR_CLAMP);
        need(name, !mirror || self.mirror_clamp, "mirror_clamp")
    }

    pub(crate) fn check_depth_state(&self, info: &DepthStateInfo) {
        let c = info.compare;
        assert_known(
            "create_depth_state",
            "compare",
            compare_op::name(c),
            compare_op::valid(c),
        );
        check_stencil_face(&info.front);
        check_stencil_face(&info.back);
    }

    pub(crate) fn check_query_pool(&self, kind: u32, count: u32) -> Result<(), String> {
        let name = "create_query_pool";
        assert_known(
            name,
            "kind",
            query_kind::name(kind),
            query_kind::valid(kind),
        );
        assert!(count > 0, "{name} count is 0");
        need(name, count <= self.max_queries, "max_queries")?;
        need(
            name,
            kind != query_kind::TIMESTAMP || self.timestamps,
            "timestamps",
        )?;
        need(name, kind != query_kind::PRECISE || self.precise, "precise")?;
        need(
            name,
            kind != query_kind::OCCLUSION || self.occlusion,
            "occlusion",
        )
    }
}
