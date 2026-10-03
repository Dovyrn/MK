use crate::{
    Caps, ColorTarget, DepthInfo, SamplerInfo, StencilFace, TextureInfo, address, blend, border,
    compare, dimension, fill, filter, format, mask, memory, op, query, stencil, topology, usage,
};

fn known(what: &str, value: u32, valid: bool) {
    assert!(valid, "unknown {what} {value}");
}

fn real(f: u32) -> bool {
    f > 0 && format::block(f)[0] > 0
}

fn samples(n: u32) {
    assert!(
        n.is_power_of_two() && n <= 64,
        "{n} samples is not a power of two up to 64"
    );
}

fn compressed(f: u32) -> bool {
    let [_, w, h] = format::block(f);
    w > 1 || h > 1
}

fn depth(f: u32) -> bool {
    (format::D16_UNORM..=format::S8_UINT).contains(&f)
}

pub(crate) fn supports(
    format: u32,
    usage: u32,
    samples: u32,
    usable: u32,
    counts: u32,
) -> Result<(), String> {
    if usable == 0 {
        return Err(format!("format {format} is not supported"));
    }
    let missing = usage & !usable;
    if missing != 0 {
        return Err(format!(
            "format {format} does not support usage {missing:#x}"
        ));
    }
    if counts & samples == 0 {
        return Err(format!(
            "format {format} does not support {samples} samples with usage {usage:#x}"
        ));
    }
    Ok(())
}

pub(crate) fn targets(colors: &[ColorTarget], depth: u32, stencil: u32, count: u32, mode: u32) {
    for c in colors {
        assert!(real(c.format), "unknown color format {}", c.format);
        for b in [c.src, c.dst, c.src_alpha, c.dst_alpha] {
            known("blend factor", b, blend::valid(b));
        }
        known("blend op", c.op, op::valid(c.op));
        known("blend op", c.op_alpha, op::valid(c.op_alpha));
        assert!(c.mask & !mask::BITS == 0, "unknown mask bits {:#x}", c.mask);
    }
    for f in [depth, stencil] {
        assert!(f == format::NONE || real(f), "unknown format {f}");
    }
    samples(count);
    known("fill", mode, fill::valid(mode));
}

pub(crate) fn primitive(t: u32) {
    known("topology", t, topology::valid(t));
}

fn face(f: &StencilFace) {
    for o in [f.fail, f.depth_fail, f.pass] {
        known("stencil op", o, stencil::valid(o));
    }
    known("compare", f.compare, compare::valid(f.compare));
}

impl Caps {
    pub(crate) fn buffer(&self, size: u64, memory: u32) -> Result<(), String> {
        known("memory", memory, memory::valid(memory));
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        assert!(size > 0, "buffer size is 0");
        if size > self.max_buffer {
            return Err(format!("buffer size {size} is over {}", self.max_buffer));
        }
        Ok(())
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub(crate) fn host(&self, pointer: *mut u8, size: u64) {
        let a = self.host_alignment.max(1) as u64;
        assert!(
            (pointer as u64).is_multiple_of(a) && size.is_multiple_of(a) && size > 0,
            "host memory {pointer:?} and size {size} must be non-zero multiples of caps.host_alignment {a}"
        );
    }

    pub(crate) fn texture(
        &self,
        info: &TextureInfo,
        usable: u32,
        counts: u32,
    ) -> Result<(), String> {
        let sizes = [info.width, info.height, info.depth, info.layers, info.mips];
        assert!(sizes.iter().all(|&n| n > 0), "texture size is 0");
        known(
            "dimension",
            info.dimension,
            dimension::valid(info.dimension),
        );
        known("memory", info.memory, memory::valid(info.memory));
        assert!(real(info.format), "unknown format {}", info.format);
        assert!(
            info.usage & !usage::VIEW != 0,
            "texture usage {:#x} needs more than usage::VIEW",
            info.usage
        );
        assert!(
            info.usage & !usage::BITS == 0,
            "unknown usage bits {:#x}",
            info.usage & !usage::BITS
        );
        samples(info.samples);
        let largest = info.width.max(info.height).max(info.depth);
        assert!(
            info.mips <= 32 - largest.leading_zeros(),
            "{} mips is more than a {largest} texture has",
            info.mips
        );
        let (w, h, d, l) = (info.width, info.height, info.depth, info.layers);
        match info.dimension {
            dimension::D1 | dimension::D1_ARRAY => assert!(
                h == 1 && d == 1 && info.mips == 1,
                "a 1D texture has height 1, depth 1 and one mip"
            ),
            dimension::D3 => assert!(l == 1, "a 3D texture has one layer"),
            dimension::CUBE | dimension::CUBE_ARRAY => assert!(
                w == h && d == 1 && l.is_multiple_of(6),
                "a cube texture has square faces, depth 1 and a multiple of 6 layers"
            ),
            _ => assert!(d == 1, "a 2D texture has depth 1"),
        }
        if matches!(
            info.dimension,
            dimension::D1 | dimension::D2 | dimension::D3
        ) {
            assert!(l == 1, "a texture without layers has one layer");
        }
        if info.dimension == dimension::CUBE {
            assert!(l == 6, "a cube texture has 6 layers");
        }
        assert!(
            info.usage & usage::HOST == 0 || info.samples == 1,
            "a usage::HOST texture has one sample"
        );
        if info.samples > 1 {
            assert!(
                matches!(info.dimension, dimension::D2 | dimension::D2_ARRAY) && info.mips == 1,
                "a multisampled texture is 2D with one mip"
            );
        }
        if compressed(info.format) || depth(info.format) {
            assert!(
                !matches!(
                    info.dimension,
                    dimension::D1 | dimension::D1_ARRAY | dimension::D3
                ),
                "format {} is not allowed in a 1D or 3D texture",
                info.format
            );
        }
        if info.memory == memory::TRANSIENT {
            assert!(
                info.usage & !(usage::COLOR | usage::DEPTH) == 0,
                "transient textures are attachments only"
            );
            if !self.memoryless {
                return Err("transient textures need caps.memoryless".into());
            }
        }
        if info.dimension == dimension::CUBE_ARRAY && !self.cube_arrays {
            return Err("a cube array needs caps.cube_arrays".into());
        }
        let side = w.max(h);
        let (size, limit) = match info.dimension {
            dimension::D3 => (largest, self.max_volume),
            dimension::CUBE | dimension::CUBE_ARRAY => (side, self.max_cube),
            _ => (side, self.max_texture),
        };
        if size > limit {
            return Err(format!("texture size {size} is over {limit}"));
        }
        if l > self.max_layers {
            return Err(format!("{l} layers is over {}", self.max_layers));
        }
        supports(info.format, info.usage, info.samples, usable, counts)
    }

    pub(crate) fn view(&self, kind: u32) -> Result<(), String> {
        if kind == dimension::CUBE_ARRAY && !self.cube_arrays {
            return Err("a cube array needs caps.cube_arrays".into());
        }
        Ok(())
    }

    pub(crate) fn sampler(&self, info: &SamplerInfo) -> Result<(), String> {
        for f in [info.min, info.mag, info.mip] {
            known("filter", f, filter::valid(f));
        }
        for a in [info.u, info.v, info.w] {
            known("address", a, address::valid(a));
        }
        known("border", info.border, border::valid(info.border));
        known("compare", info.compare, compare::valid(info.compare));
        let mirror = [info.u, info.v, info.w].contains(&address::MIRROR_CLAMP);
        if mirror && !self.mirror_clamp {
            return Err("address::MIRROR_CLAMP needs caps.mirror_clamp".into());
        }
        Ok(())
    }

    pub(crate) fn depth(&self, info: &DepthInfo) {
        known("compare", info.compare, compare::valid(info.compare));
        face(&info.front);
        face(&info.back);
    }

    pub(crate) fn queries(&self, kind: u32, count: u32) -> Result<(), String> {
        known("query kind", kind, query::valid(kind));
        assert!(count > 0, "a query heap needs at least one query");
        if count > self.max_queries {
            return Err(format!(
                "{count} queries is over caps.max_queries {}",
                self.max_queries
            ));
        }
        Ok(())
    }
}
