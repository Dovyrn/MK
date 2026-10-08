use crate::entries::Entries;
use crate::handle::TimelineId;
use crate::residency::{Members, Residency};
use crate::{
    Buffer, CommandPool, ComputePipeline, DepthState, HostTextureCopy, IndirectBuffer,
    IndirectBufferInfo, QueryPool, RenderPipeline, ResidencySet, Sampler, Texture, TextureInfo,
    TextureViewInfo, TilePipeline, format, index_type, memory, texture_dimension, texture_usage,
};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, Weak};

#[derive(Clone, Copy)]
pub struct RenderEntry {
    pub samples: u32,
    pub topology: u32,
    pub mesh: bool,
    pub indirect: bool,
    pub vertices: u32,
    pub reads: u32,
}

pub type Signal = (u64, usize, Weak<()>);

#[derive(Default)]
pub struct Table {
    pub buffers: Entries<Buffer, u64>,
    pub textures: Entries<Texture, TextureInfo>,
    pub samplers: Entries<Sampler, ()>,
    pub pipelines: Entries<ComputePipeline, ()>,
    pub renders: Entries<RenderPipeline, RenderEntry>,
    pub tile_pipelines: Entries<TilePipeline, u32>,
    pub states: Entries<DepthState, ()>,
    pub queries: Entries<QueryPool, (u32, u32)>,
    pub timelines: RefCell<Entries<TimelineId, Signal>>,
    pub parents: Entries<Texture, Texture>,
    pub pools: Entries<CommandPool, (usize, Cell<bool>)>,
    pub undefined: RefCell<Vec<Texture>>,
    pub residency: Residency,
    pub indirects: Entries<IndirectBuffer, IndirectEntry>,
    pub mappable: Entries<Buffer, bool>,
}

#[derive(Clone, Copy)]
pub struct IndirectEntry {
    pub info: IndirectBufferInfo,
    pub topology: u32,
    pub mesh: bool,
}

fn rest(base: u32, count: u32, total: u32, what: &str, texture: Texture) -> u32 {
    check!(
        base < total,
        "create_texture_view base {what} {base} is outside the {total} {what}s of {texture}"
    );
    let count = if count == !0 { total - base } else { count };
    check!(
        count > 0 && count <= total - base,
        "create_texture_view {what}s {base}..{} are outside the {total} {what}s of {texture}",
        base as u64 + count as u64
    );
    count
}

pub fn vertex_buffer_mask(info: &crate::RenderPipelineInfo) -> u32 {
    info.attributes.iter().fold(0, |m, a| m | 1 << a.buffer)
}

impl Table {
    pub fn drop_buffer(&mut self, buffer: Buffer) {
        self.buffers.get(buffer);
        self.residency.free_buffer(buffer);
        self.buffers.clear(buffer);
    }

    pub fn check_indirect(&self, info: &IndirectBufferInfo) -> IndirectEntry {
        let name = "create_indirect_buffer";
        check!(info.draws > 0, "{name} draws is 0");
        let pipeline = info.pipeline;
        let entry = *self.renders.get(pipeline);
        check!(
            entry.indirect,
            "{name} needs {pipeline} created with indirect: true"
        );
        if info.indexed {
            check!(!entry.mesh, "{name} of mesh {pipeline} cannot be indexed");
            let kind = info.kind;
            check!(
                index_type::valid(kind),
                "{name} kind is {}",
                index_type::name(kind)
            );
            let size = if kind == index_type::U16 { 2 } else { 4 };
            check!(
                info.offset.is_multiple_of(size),
                "{name} index offset {} is not a multiple of {size}",
                info.offset
            );
            let total = *self.buffers.get(info.indices);
            check!(
                info.offset < total,
                "{name} index offset {} is past {} ({total} bytes)",
                info.offset,
                info.indices
            );
        }
        IndirectEntry {
            info: *info,
            topology: entry.topology,
            mesh: entry.mesh,
        }
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn shader_handle(
        &self,
        indirect: IndirectBuffer,
        commands: u64,
        indices: u64,
        groups: [u32; 6],
    ) -> crate::IndirectShaderHandle {
        let e = *self.indirects.get(indirect);
        let mut words = [commands as u32, (commands >> 32) as u32, 0, 0, 0, 0, 0, 0];
        if e.mesh {
            words[2..].copy_from_slice(&groups);
        } else if e.info.indexed {
            let kind = e.info.kind;
            words[2..6].copy_from_slice(&[
                indices as u32,
                (indices >> 32) as u32,
                e.topology,
                kind,
            ]);
        } else {
            words[2] = e.topology;
        }
        crate::IndirectShaderHandle(words)
    }

    pub fn drop_indirect(&mut self, indirect: IndirectBuffer) {
        self.indirects.get(indirect);
        self.residency.free_indirect(indirect);
        self.indirects.clear(indirect);
    }

    pub fn add_buffer(&mut self, buffer: Buffer, size: u64, mappable: bool) {
        self.buffers.set(buffer, size);
        self.mappable.set(buffer, mappable);
    }

    pub fn assert_mappable(&self, buffer: Buffer) {
        check!(
            *self.mappable.get(buffer),
            "map_buffer on {buffer}, which has no host mapping; create it with memory::UPLOAD or memory::READBACK, or import a mapped buffer"
        );
    }

    pub fn drop_query(&mut self, pool: QueryPool) {
        self.queries.get(pool);
        self.residency.free_query(pool);
        self.queries.clear(pool);
    }

    pub fn change_residency(&mut self, set: ResidencySet, items: Members, add: bool) {
        for &b in &items.buffers {
            self.buffers.get(b);
        }
        for &t in &items.textures {
            self.textures.get(t);
            check!(
                self.parents.find(t).is_none(),
                "add_residency and remove_residency take whole textures, not view {t}"
            );
            check_ne!(
                self.textures.get(t).memory,
                memory::TRANSIENT,
                "{t} is memory::TRANSIENT and has no memory to make resident"
            );
        }
        for &q in &items.queries {
            self.queries.get(q);
        }
        for &i in &items.indirects {
            self.indirects.get(i);
        }
        self.residency.change(set, &items, add);
    }

    pub fn add_texture(&mut self, texture: Texture, info: &TextureInfo) {
        self.textures.set(texture, *info);
        if info.memory == memory::TRANSIENT {
            self.residency.exempt_texture(texture);
        }
        if info.usage & texture_usage::HOST == 0 {
            self.undefined.get_mut().push(texture);
        }
    }

    pub fn define(&self, texture: Texture) {
        self.undefined.borrow_mut().retain(|&t| t != texture);
    }

    pub fn assert_defined(&self, name: &str, texture: Texture) {
        let undefined = self.undefined.borrow();
        if undefined.is_empty() {
            return;
        }
        let root = self.parents.find(texture).copied().unwrap_or(texture);
        check!(
            !undefined.contains(&root),
            "{name} uses {texture} before its first texture_barrier; record one with before: texture_layout::UNDEFINED"
        );
    }

    pub fn drop_texture(&mut self, texture: Texture) {
        check!(
            !self.parents.values().any(|&p| p == texture),
            "destroy_texture on {texture} while views of it live; destroy the views first"
        );
        self.textures.get(texture);
        if self.parents.find(texture).is_none() {
            self.residency.free_texture(texture);
        }
        self.textures.clear(texture);
        self.parents.forget(texture);
        self.undefined.get_mut().retain(|&t| t != texture);
    }

    pub fn resolve_view(&self, texture: Texture, info: &TextureViewInfo) -> TextureViewInfo {
        let t = self.textures.get(texture);
        check!(
            texture_dimension::valid(info.dimension),
            "unknown dimension {}",
            info.dimension
        );
        check!(
            info.format > 0 && format::block(info.format)[0] > 0,
            "unknown format {}",
            info.format
        );
        check!(
            info.usage != 0 && info.usage & !t.usage == 0,
            "view usage {:#x} is not part of {texture} usage {:#x}",
            info.usage,
            t.usage
        );
        check!(
            info.format == t.format || t.usage & texture_usage::VIEW != 0,
            "a view of {texture} in format {} needs texture_usage::VIEW",
            info.format
        );
        check_eq!(
            format::block(info.format),
            format::block(t.format),
            "format {} has a different block than format {}",
            info.format,
            t.format
        );
        let mips = rest(info.base_mip, info.mips, t.mips, "mip", texture);
        let layers = rest(info.base_layer, info.layers, t.layers, "layer", texture);
        let family = |d: u32| match d {
            texture_dimension::D1 | texture_dimension::D1_ARRAY => 1,
            texture_dimension::D3 => 3,
            _ => 2,
        };
        check_eq!(
            family(t.dimension),
            family(info.dimension),
            "a view of {texture} cannot change dimension {} to {}",
            t.dimension,
            info.dimension
        );
        let cube = matches!(
            t.dimension,
            texture_dimension::CUBE | texture_dimension::CUBE_ARRAY
        );
        match info.dimension {
            texture_dimension::D1 | texture_dimension::D2 | texture_dimension::D3 => {
                check_eq!(
                    layers,
                    1,
                    "a view without layers has one layer, not {layers}"
                )
            }
            texture_dimension::CUBE => check!(
                cube && layers == 6,
                "a cube view needs a cube texture and 6 layers"
            ),
            texture_dimension::CUBE_ARRAY => check!(
                cube && layers.is_multiple_of(6),
                "a cube array view needs a cube texture and a multiple of 6 layers"
            ),
            _ => {}
        }
        TextureViewInfo {
            mips,
            layers,
            ..*info
        }
    }

    pub fn check_view(&mut self, id: Texture, texture: Texture, info: &TextureViewInfo) {
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

    pub fn check_host_copy(&self, c: &HostTextureCopy, size: usize) -> (u64, u64) {
        let texture = c.texture;
        let t = self.textures.get(texture);
        check_ne!(
            t.usage & texture_usage::HOST,
            0,
            "write_texture and read_texture need {texture} created with texture_usage::HOST"
        );
        check!(
            t.samples == 1 && !(format::D16_UNORM..=format::S8_UINT).contains(&t.format),
            "write_texture and read_texture need a single-sampled color texture, not {texture}"
        );
        let (w, h, d) = (c.width, c.height, c.depth);
        check!(
            w > 0 && h > 0 && d > 0,
            "host copy region {w}x{h}x{d} has a zero"
        );
        check!(
            c.mip < t.mips && c.layer < t.layers,
            "mip {} layer {} is outside {} mips and {} layers of {texture}",
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
        check!(
            fits(c.x, w, sizes[0]) && fits(c.y, h, sizes[1]) && fits(c.z, d, sizes[2]),
            "region {},{},{} {w}x{h}x{d} is outside mip {} of {texture} ({}x{}x{})",
            c.x,
            c.y,
            c.z,
            c.mip,
            sizes[0],
            sizes[1],
            sizes[2]
        );
        check!(
            edge(c.x, w, sizes[0], bw) && edge(c.y, h, sizes[1], bh),
            "region {},{} {w}x{h} is not aligned to the {bw}x{bh} blocks of {texture}",
            c.x,
            c.y
        );
        let row = if c.row_length == 0 { w } else { c.row_length };
        let rows = if c.row_count == 0 { h } else { c.row_count };
        check!(
            row >= w && (row.is_multiple_of(bw) || row == w),
            "row length {row} is below width {w} or not a multiple of block width {bw}"
        );
        check!(
            rows >= h && (rows.is_multiple_of(bh) || rows == h),
            "row count {rows} is below height {h} or not a multiple of block height {bh}"
        );
        let pitch = row.div_ceil(bw) as u64 * format::block(t.format)[0] as u64;
        let image = pitch * rows.div_ceil(bh) as u64;
        let need = image
            .checked_mul(d as u64)
            .expect("host copy size overflows");
        check!(
            size as u64 >= need,
            "host copy needs {need} bytes, not {size}"
        );
        (pitch, image)
    }

    pub fn add_timeline(&mut self, id: TimelineId) -> Arc<()> {
        let token = Arc::new(());
        self.timelines
            .get_mut()
            .set(id, (0, usize::MAX, Arc::downgrade(&token)));
        token
    }

    pub fn drop_timeline(&mut self, timeline: &crate::Timeline) {
        self.timeline_points(&[(timeline, 0)]);
        self.timelines.get_mut().clear(timeline.id);
    }

    pub fn timeline_points(&self, list: &[(&crate::Timeline, u64)]) -> Vec<(TimelineId, u64)> {
        let timelines = self.timelines.borrow();
        list.iter()
            .map(|&(t, value)| {
                let (_, _, owner) = timelines.get(t.id);
                check!(
                    Weak::ptr_eq(owner, &Arc::downgrade(&t.token)),
                    "{} belongs to another device; use the device that created it",
                    t.id
                );
                (t.id, value)
            })
            .collect()
    }

    pub fn check_signals(&self, queue: usize, signals: &[(TimelineId, u64)]) {
        let timelines = self.timelines.borrow();
        for (i, &(id, value)) in signals.iter().enumerate() {
            let (last, from, _) = timelines.get(id);
            check!(
                *from != queue || value > *last,
                "submit signals {id} to {value}, but queue {queue} already signalled it to {last}; signal a value above {last}"
            );
            check!(
                signals[..i].iter().all(|s| s.0 != id),
                "submit signals {id} twice; signal it once with the highest value"
            );
        }
    }

    pub fn record_signals(&self, queue: usize, signals: &[(TimelineId, u64)]) {
        let mut timelines = self.timelines.borrow_mut();
        for &(id, value) in signals {
            let entry = timelines.get_mut(id);
            entry.0 = value;
            entry.1 = queue;
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            check!(
                self.timelines
                    .get_mut()
                    .values()
                    .all(|t| t.2.strong_count() == 0),
                "a timeline outlives its device; drop or destroy_timeline every timeline before the device"
            );
        }
    }
}

pub fn assert_rising(now: u64, value: u64) {
    check!(
        value > now,
        "signal {value} is not above the timeline's value {now}; signal a value above {now}"
    );
}
