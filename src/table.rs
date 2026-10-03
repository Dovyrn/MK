use crate::entries::Entries;
use crate::{
    HostCopy, IndirectInfo, TextureInfo, ViewInfo, dimension, format, index, layout, usage,
};

pub struct Table {
    pub buffers: Entries<u64>,
    pub textures: Entries<TextureInfo>,
    pub samplers: Entries<()>,
    pub pipelines: Entries<()>,
    pub renders: Entries<(u32, bool, u32, bool)>,
    pub states: Entries<()>,
    pub queries: Entries<(u32, u32)>,
    pub events: Entries<(u64, usize)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub heaps: Entries<u64>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub accels: Entries<(u64, u64)>,
    pub indirects: Entries<(IndirectInfo, u32, bool)>,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            buffers: Entries::new("buffer"),
            textures: Entries::new("texture"),
            samplers: Entries::new("sampler"),
            pipelines: Entries::new("pipeline"),
            renders: Entries::new("render pipeline"),
            states: Entries::new("depth state"),
            queries: Entries::new("query heap"),
            events: Entries::new("event"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            heaps: Entries::new("heap"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            accels: Entries::new("acceleration structure"),
            indirects: Entries::new("indirect buffer"),
        }
    }
}

#[cfg(any(feature = "vk", feature = "gl"))]
pub fn stride(mesh: bool, indexed: bool) -> u32 {
    match (mesh, indexed) {
        (true, _) => 12,
        (_, true) => 20,
        _ => 16,
    }
}

fn rest(base: u32, count: u32, total: u32, what: &str, texture: u32) -> u32 {
    assert!(
        base < total,
        "base {what} {base} is outside 0..{total} of texture {texture}"
    );
    let count = if count == !0 { total - base } else { count };
    assert!(
        count > 0 && count <= total - base,
        "{what}s {base}..{} are outside 0..{total} of texture {texture}",
        base as u64 + count as u64
    );
    count
}

impl Table {
    pub fn indirect(&self, info: &IndirectInfo) -> (IndirectInfo, u32, bool) {
        assert!(info.size > 0, "indirect buffer size is 0");
        let pipeline = info.pipeline;
        let (_, indirect, topology, mesh) = *self.renders.get(pipeline);
        assert!(
            indirect,
            "an indirect buffer needs render pipeline {pipeline} created with indirect: true"
        );
        if info.indexed {
            assert!(
                !mesh,
                "an indirect buffer of mesh pipeline {pipeline} is not indexed"
            );
            assert!(index::valid(info.kind), "unknown index kind {}", info.kind);
            let total = *self.buffers.get(info.indices);
            assert!(
                info.offset < total,
                "index offset {} is past buffer {} ({total} bytes)",
                info.offset,
                info.indices
            );
        }
        (*info, topology, mesh)
    }

    pub fn handle(&self, indirect: u32, commands: u64, indices: u64, groups: [u32; 6]) -> [u32; 8] {
        let (info, topology, mesh) = *self.indirects.get(indirect);
        let mut words = [commands as u32, (commands >> 32) as u32, 0, 0, 0, 0, 0, 0];
        if mesh {
            words[2..].copy_from_slice(&groups);
        } else if info.indexed {
            let kind = info.kind;
            words[2..6].copy_from_slice(&[indices as u32, (indices >> 32) as u32, topology, kind]);
        } else {
            words[2] = topology;
        }
        words
    }

    pub fn resolve(&self, texture: u32, info: &ViewInfo) -> ViewInfo {
        let t = self.textures.get(texture);
        assert!(
            dimension::valid(info.dimension),
            "unknown dimension {}",
            info.dimension
        );
        assert!(
            info.format > 0 && format::block(info.format)[0] > 0,
            "unknown format {}",
            info.format
        );
        assert!(
            info.usage != 0 && info.usage & !t.usage == 0,
            "view usage {:#x} is not part of texture {texture} usage {:#x}",
            info.usage,
            t.usage
        );
        assert!(
            info.format == t.format || t.usage & usage::VIEW != 0,
            "a view of texture {texture} in format {} needs usage::VIEW",
            info.format
        );
        assert_eq!(
            format::block(info.format),
            format::block(t.format),
            "format {} has a different block than format {}",
            info.format,
            t.format
        );
        let mips = rest(info.base_mip, info.mips, t.mips, "mip", texture);
        let layers = rest(info.base_layer, info.layers, t.layers, "layer", texture);
        let family = |d: u32| match d {
            dimension::D1 | dimension::D1_ARRAY => 1,
            dimension::D3 => 3,
            _ => 2,
        };
        assert_eq!(
            family(t.dimension),
            family(info.dimension),
            "a view of texture {texture} cannot change dimension {} to {}",
            t.dimension,
            info.dimension
        );
        let cube = matches!(t.dimension, dimension::CUBE | dimension::CUBE_ARRAY);
        match info.dimension {
            dimension::D1 | dimension::D2 | dimension::D3 => {
                assert_eq!(
                    layers, 1,
                    "a view without layers has one layer, not {layers}"
                )
            }
            dimension::CUBE => assert!(
                cube && layers == 6,
                "a cube view needs a cube texture and 6 layers"
            ),
            dimension::CUBE_ARRAY => assert!(
                cube && layers.is_multiple_of(6),
                "a cube array view needs a cube texture and a multiple of 6 layers"
            ),
            _ => {}
        }
        ViewInfo {
            mips,
            layers,
            ..*info
        }
    }

    pub fn view(&mut self, id: u32, texture: u32, info: &ViewInfo) {
        let t = *self.textures.get(texture);
        self.textures.set(
            id,
            TextureInfo {
                dimension: info.dimension,
                format: info.format,
                width: (t.width >> info.base_mip).max(1),
                height: (t.height >> info.base_mip).max(1),
                depth: (t.depth >> info.base_mip).max(1),
                layers: info.layers,
                mips: info.mips,
                usage: info.usage,
                ..t
            },
        );
    }

    pub fn host(&self, c: &HostCopy, size: usize) -> (u64, u64) {
        let texture = c.texture;
        let t = self.textures.get(texture);
        assert_ne!(
            t.usage & usage::HOST,
            0,
            "texture {texture} has no usage::HOST"
        );
        assert!(
            t.samples == 1 && !(format::D16_UNORM..=format::S8_UINT).contains(&t.format),
            "host copies need a single-sampled color texture, not texture {texture}"
        );
        assert!(
            layout::valid(c.layout) && c.layout != layout::UNDEFINED,
            "host copy layout {} is not a defined layout",
            c.layout
        );
        let (w, h, d) = (c.width, c.height, c.depth);
        assert!(
            w > 0 && h > 0 && d > 0,
            "host copy region {w}x{h}x{d} has a zero"
        );
        assert!(
            c.mip < t.mips && c.layer < t.layers,
            "mip {} layer {} is outside {} mips and {} layers of texture {texture}",
            c.mip,
            c.layer,
            t.mips,
            t.layers
        );
        let sizes = [t.width, t.height, t.depth].map(|n| (n >> c.mip).max(1));
        let [_, bw, bh] = format::block(t.format);
        let fits = |at: u32, n: u32, end: u32| at as u64 + n as u64 <= end as u64;
        let edge = |at: u32, n: u32, end: u32, b: u32| {
            at.is_multiple_of(b) && (n.is_multiple_of(b) || at + n == end)
        };
        assert!(
            fits(c.x, w, sizes[0]) && fits(c.y, h, sizes[1]) && fits(c.z, d, sizes[2]),
            "region {},{},{} {w}x{h}x{d} is outside mip {} of texture {texture} ({}x{}x{})",
            c.x,
            c.y,
            c.z,
            c.mip,
            sizes[0],
            sizes[1],
            sizes[2]
        );
        assert!(
            edge(c.x, w, sizes[0], bw) && edge(c.y, h, sizes[1], bh),
            "region {},{} {w}x{h} is not aligned to the {bw}x{bh} blocks of texture {texture}",
            c.x,
            c.y
        );
        let row = if c.row_length == 0 { w } else { c.row_length };
        let rows = if c.row_count == 0 { h } else { c.row_count };
        assert!(
            row >= w && (row.is_multiple_of(bw) || row == w),
            "row length {row} is below width {w} or not a multiple of block width {bw}"
        );
        assert!(
            rows >= h && (rows.is_multiple_of(bh) || rows == h),
            "row count {rows} is below height {h} or not a multiple of block height {bh}"
        );
        let pitch = row.div_ceil(bw) as u64 * format::block(t.format)[0] as u64;
        let image = pitch * rows.div_ceil(bh) as u64;
        assert!(
            size as u64 >= image * d as u64,
            "host copy needs {} bytes, not {size}",
            image * d as u64
        );
        (pitch, image)
    }

    pub fn submit(&self, queue: usize, waits: &[(u32, u64)], signals: &[(u32, u64)]) {
        for &(event, _) in waits {
            self.events.get(event);
        }
        for (i, &(event, value)) in signals.iter().enumerate() {
            let (last, from) = *self.events.get(event);
            assert!(
                from != queue || value > last,
                "event {event} is already signalled to {last} on queue {queue}, not below {value}"
            );
            assert!(
                signals[..i].iter().all(|s| s.0 != event),
                "event {event} is signalled twice in one submit"
            );
        }
    }

    pub fn signal(&mut self, queue: usize, signals: &[(u32, u64)]) {
        for &(event, value) in signals {
            self.events.set(event, (value, queue));
        }
    }
}
