use crate::entries::Entries;
use crate::{
    HostCopy, IndirectInfo, TextureInfo, ViewInfo, dimension, format, index, layout, usage,
};
use std::sync::{Arc, Weak};

pub struct Table {
    pub buffers: Entries<u64>,
    pub textures: Entries<TextureInfo>,
    pub samplers: Entries<()>,
    pub pipelines: Entries<()>,
    pub renders: Entries<(u32, bool, u32, bool, u32)>,
    pub states: Entries<()>,
    pub queries: Entries<(u32, u32)>,
    pub timelines: Entries<(u64, usize, Weak<()>)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub heaps: Entries<u64>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub accels: Entries<(u64, u64)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub geometry: Entries<(Vec<u32>, bool)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub buffer_heaps: Entries<u32>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub texture_heaps: Entries<u32>,
    pub parents: Entries<u32>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub rays: Entries<(u32, u32)>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub pages: Entries<()>,
    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub tiles: Entries<[u32; 3]>,
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
            timelines: Entries::new("timeline"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            heaps: Entries::new("heap"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            accels: Entries::new("acceleration structure"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            geometry: Entries::new("acceleration structure"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            buffer_heaps: Entries::new("placed buffer"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            texture_heaps: Entries::new("placed texture"),
            parents: Entries::new("view"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            rays: Entries::new("ray instances"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            pages: Entries::new("sparse buffer"),
            #[cfg(any(feature = "vk", feature = "mtl"))]
            tiles: Entries::new("sparse texture"),
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

#[cfg(any(feature = "vk", feature = "mtl"))]
pub fn geometry(info: &crate::AccelInfo) -> Vec<u32> {
    let mut buffers = Vec::new();
    for t in info.triangles {
        buffers.push(t.vertices);
        if t.indices != !0 {
            buffers.push(t.indices);
        }
    }
    buffers.extend(info.boxes.iter().map(|b| b.buffer));
    buffers
}

pub fn vertices(info: &crate::RenderInfo) -> u32 {
    info.attributes.iter().fold(0, |m, a| m | 1 << a.buffer)
}

impl Table {
    pub fn drop_buffer(&mut self, buffer: u32) {
        self.buffers.clear(buffer);
        #[cfg(any(feature = "vk", feature = "mtl"))]
        {
            self.pages.forget(buffer);
            self.buffer_heaps.forget(buffer);
            for (buffers, intact) in self.geometry.values_mut() {
                if buffers.contains(&buffer) {
                    *intact = false;
                }
            }
        }
    }

    pub fn drop_texture(&mut self, texture: u32) {
        assert!(
            !self.parents.values().any(|&p| p == texture),
            "texture {texture} is destroyed while views of it live"
        );
        self.textures.clear(texture);
        self.parents.forget(texture);
        #[cfg(any(feature = "vk", feature = "mtl"))]
        {
            self.tiles.forget(texture);
            self.texture_heaps.forget(texture);
        }
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn drop_heap(&mut self, heap: u32) {
        let placed = self
            .buffer_heaps
            .values()
            .chain(self.texture_heaps.values());
        assert!(
            !placed.into_iter().any(|&h| h == heap),
            "heap {heap} is destroyed while resources placed in it live"
        );
        self.heaps.clear(heap);
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn ray_write(&self, instances: u32, index: u32, id: u32, mask: u32, flags: u32) {
        let (count, memory) = *self.rays.get(instances);
        assert!(index < count, "ray instance {index} is outside 0..{count}");
        assert_eq!(
            memory,
            crate::memory::UPLOAD,
            "write_ray_instance needs ray instances created with memory::UPLOAD"
        );
        assert!(id < 1 << 24, "ray instance id {id} is over 24 bits");
        assert!(
            mask < 256 && flags & !crate::instance::BITS == 0,
            "ray instance mask {mask} is over 8 bits or flags {flags:#x} are unknown"
        );
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn accel(&mut self, id: u32, sizes: (u64, u64), buffers: Vec<u32>) {
        self.accels.set(id, sizes);
        self.geometry.set(id, (buffers, true));
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn drop_accel(&mut self, accel: u32) {
        self.accels.clear(accel);
        self.geometry.forget(accel);
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    fn heap_pages(&self, heap: u32, page: u64) -> u64 {
        if heap == !0 {
            u64::MAX
        } else {
            *self.heaps.get(heap) / page
        }
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn map_buffer(&self, buffer: u32, heap: u32, page: u64, ranges: &[crate::SparseRange]) {
        self.pages.get(buffer);
        let pages = self.buffers.get(buffer).div_ceil(page);
        let room = self.heap_pages(heap, page);
        for r in ranges {
            let end = r.page as u64 + r.count as u64;
            assert!(
                r.count > 0 && end <= pages,
                "sparse pages {}..{end} are outside the {pages} pages of buffer {buffer}",
                r.page
            );
            assert!(
                r.heap_page as u64 + r.count as u64 <= room,
                "heap pages {}..{} are outside heap {heap}",
                r.heap_page,
                r.heap_page as u64 + r.count as u64
            );
        }
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn map_texture(&self, texture: u32, heap: u32, page: u64, regions: &[crate::SparseRegion]) {
        let tile = *self.tiles.get(texture);
        let t = self.textures.get(texture);
        let room = self.heap_pages(heap, page);
        for r in regions {
            assert!(
                r.mip < t.mips && r.layer < t.layers,
                "sparse region mip {} layer {} is outside texture {texture}",
                r.mip,
                r.layer
            );
            let size = [t.width, t.height, t.depth].map(|n| (n >> r.mip).max(1));
            let grid: Vec<u64> = (0..3).map(|i| size[i].div_ceil(tile[i]) as u64).collect();
            let at = [r.x, r.y, r.z];
            let span = [r.width, r.height, r.depth];
            assert!(
                (0..3).all(|i| span[i] > 0 && at[i] as u64 + span[i] as u64 <= grid[i]),
                "sparse region {at:?} {span:?} is outside the {grid:?} tiles of mip {} of texture {texture}",
                r.mip
            );
            let count = span.iter().map(|&n| n as u64).product::<u64>();
            assert!(
                r.heap_page as u64 + count <= room,
                "heap pages {}..{} are outside heap {heap}",
                r.heap_page,
                r.heap_page as u64 + count
            );
        }
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn place(&self, heap: u32, offset: u64, (size, align): (u64, u64)) {
        let total = *self.heaps.get(heap);
        assert!(
            offset.is_multiple_of(align),
            "offset {offset} in heap {heap} is not a multiple of the alignment {align}"
        );
        assert!(
            offset.checked_add(size).is_some_and(|end| end <= total),
            "{size} bytes at offset {offset} run past heap {heap} ({total} bytes)"
        );
    }

    pub fn indirect(&self, info: &IndirectInfo) -> (IndirectInfo, u32, bool) {
        assert!(info.size > 0, "indirect buffer size is 0");
        let pipeline = info.pipeline;
        let (_, indirect, topology, mesh, _) = *self.renders.get(pipeline);
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
            let size = if info.kind == index::U16 { 2 } else { 4 };
            assert!(
                info.offset.is_multiple_of(size),
                "index offset {} is not a multiple of {size}",
                info.offset
            );
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
        self.parents.set(id, texture);
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
        let need = image
            .checked_mul(d as u64)
            .expect("host copy size overflows");
        assert!(
            size as u64 >= need,
            "host copy needs {need} bytes, not {size}"
        );
        (pitch, image)
    }

    pub fn add_timeline(&mut self, id: u32) -> Arc<()> {
        let token = Arc::new(());
        self.timelines
            .set(id, (0, usize::MAX, Arc::downgrade(&token)));
        token
    }

    pub fn drop_timeline(&mut self, timeline: &crate::Timeline) {
        self.points(&[(timeline, 0)]);
        self.timelines.clear(timeline.id);
    }

    pub fn points(&self, list: &[(&crate::Timeline, u64)]) -> Vec<(u32, u64)> {
        list.iter()
            .map(|&(t, value)| {
                let (_, _, owner) = self.timelines.get(t.id);
                assert!(
                    Weak::ptr_eq(owner, &Arc::downgrade(&t.token)),
                    "timeline {} belongs to another device",
                    t.id
                );
                (t.id, value)
            })
            .collect()
    }

    pub fn submit(&self, queue: usize, signals: &[(u32, u64)]) {
        for (i, &(id, value)) in signals.iter().enumerate() {
            let (last, from, _) = self.timelines.get(id);
            assert!(
                *from != queue || value > *last,
                "timeline {id} is already signalled to {last} on queue {queue}, not below {value}"
            );
            assert!(
                signals[..i].iter().all(|s| s.0 != id),
                "timeline {id} is signalled twice in one submit"
            );
        }
    }

    pub fn signal(&mut self, queue: usize, signals: &[(u32, u64)]) {
        for &(id, value) in signals {
            let owner = self.timelines.get(id).2.clone();
            self.timelines.set(id, (value, queue, owner));
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(
                self.timelines.values().all(|t| t.2.strong_count() == 0),
                "a timeline outlives its device"
            );
        }
    }
}

pub fn rising(now: u64, value: u64) {
    assert!(
        value > now,
        "signal {value} is not above the timeline's value {now}"
    );
}
