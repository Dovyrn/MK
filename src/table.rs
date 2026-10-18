use crate::entries::Entries;
use crate::handle::TimelineId;
use crate::residency::{Members, Residency};
use crate::{
    Buffer, BufferUsage, CommandPool, ComputePipeline, DepthState, Format, HostTextureCopy,
    IndexType, IndirectBuffer, IndirectBufferInfo, Memory, QueryPool, RenderPipeline, ResidencySet,
    Sampler, Texture, TextureDimension, TextureInfo, TextureUsage, TextureViewInfo, TilePipeline,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

#[derive(Clone, Copy)]
pub struct RenderEntry {
    pub samples: u32,
    pub topology: crate::Topology,
    pub mesh: bool,
    pub indirect: bool,
    pub vertices: u32,
    pub reads: u32,
}

pub type Signal = (u64, usize, Weak<()>);

#[derive(Default)]
pub struct Table {
    pub buffers: Entries<Buffer, u64>,
    pub textures: Entries<Texture, TextureInfo<'static>>,
    pub samplers: Entries<Sampler, ()>,
    pub pipelines: Entries<ComputePipeline, ()>,
    pub renders: Entries<RenderPipeline, RenderEntry>,
    pub tile_pipelines: Entries<TilePipeline, u32>,
    pub states: Entries<DepthState, ()>,
    pub queries: Entries<QueryPool, (crate::QueryKind, u32)>,
    pub timelines: Mutex<Entries<TimelineId, Signal>>,
    pub parents: Entries<Texture, Texture>,
    pub views: Mutex<Vec<Texture>>,
    pub pools: Entries<CommandPool, (usize, AtomicBool)>,
    pub undefined: Mutex<Vec<Texture>>,
    pub residency: Mutex<Residency>,
    pub indirects: Entries<IndirectBuffer, IndirectEntry>,
    pub mappable: Entries<Buffer, bool>,
    pub usages: Entries<Buffer, BufferUsage>,
    pub presentable: Entries<Texture, AtomicBool>,
    pub shared: Entries<Texture, ()>,
}

#[derive(Clone, Copy)]
pub struct IndirectEntry {
    pub info: IndirectBufferInfo<'static>,
    pub topology: crate::Topology,
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

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn vertex_buffer_mask(info: &crate::RenderPipelineInfo) -> u32 {
    info.attributes.iter().fold(0, |m, a| m | 1 << a.buffer)
}

impl Table {
    pub fn drop_buffer(&self, buffer: Buffer) {
        self.buffers.get(buffer);
        lock(&self.residency).free_buffer(buffer);
        self.buffers.clear(buffer);
        self.mappable.clear(buffer);
        self.usages.clear(buffer);
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
            let size = if kind == IndexType::U16 { 2 } else { 4 };
            check!(
                info.offset.is_multiple_of(size),
                "{name} index offset {} is not a multiple of {size}",
                info.offset
            );
            self.assert_usage(name, info.indices, BufferUsage::INDEX, "INDEX");
            let total = *self.buffers.get(info.indices);
            check!(
                info.offset < total,
                "{name} index offset {} is past {} ({total} bytes)",
                info.offset,
                info.indices
            );
        }
        IndirectEntry {
            info: IndirectBufferInfo { label: "", ..*info },
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
                e.topology as u32,
                kind as u32,
            ]);
        } else {
            words[2] = e.topology as u32;
        }
        crate::IndirectShaderHandle(words)
    }

    pub fn drop_pool(&self, pool: CommandPool) {
        assert!(
            !self.pools.get(pool).1.swap(true, Ordering::Acquire),
            "destroy_command_pool on {pool} while it records a command buffer; end or drop that one first"
        );
        self.pools.clear(pool);
    }

    pub fn drop_indirect(&self, indirect: IndirectBuffer) {
        self.indirects.get(indirect);
        lock(&self.residency).free_indirect(indirect);
        self.indirects.clear(indirect);
    }

    pub fn add_buffer(&self, buffer: Buffer, size: u64, mappable: bool, usage: BufferUsage) {
        self.buffers.set(buffer, size);
        self.mappable.set(buffer, mappable);
        self.usages.set(buffer, usage);
    }

    pub fn assert_usage(&self, name: &str, buffer: Buffer, needs: BufferUsage, what: &str) {
        check!(
            self.usages.get(buffer).contains(needs),
            "{name} needs {buffer} created with BufferUsage::{what}"
        );
    }

    pub fn assert_mappable(&self, buffer: Buffer) {
        check!(
            *self.mappable.get(buffer),
            "map_buffer on {buffer}, which has no host mapping; create it with Memory::Upload or Memory::Readback, or import a mapped buffer"
        );
    }

    pub unsafe fn mapping<'a, T>(&self, buffer: Buffer, data: *mut u8) -> &'a mut [T] {
        let size = *self.buffers.get(buffer);
        self.assert_mappable(buffer);
        check!(
            data.cast::<T>().is_aligned(),
            "map_buffer of {buffer} as {} needs {}-byte alignment",
            std::any::type_name::<T>(),
            align_of::<T>()
        );
        let count = size as usize / size_of::<T>();
        unsafe { std::slice::from_raw_parts_mut(data.cast(), count) }
    }

    pub fn drop_query(&self, pool: QueryPool) {
        self.queries.get(pool);
        lock(&self.residency).free_query(pool);
        self.queries.clear(pool);
    }

    pub fn change_residency(&self, set: ResidencySet, items: Members, add: bool) {
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
                Memory::Transient,
                "{t} is Memory::Transient and has no memory to make resident"
            );
        }
        for &q in &items.queries {
            self.queries.get(q);
        }
        for &i in &items.indirects {
            self.indirects.get(i);
        }
        lock(&self.residency).change(set, &items, add);
    }

    pub fn add_texture(&self, texture: Texture, info: &TextureInfo) {
        self.textures
            .set(texture, TextureInfo { label: "", ..*info });
        if info.memory == Memory::Transient {
            lock(&self.residency).exempt_texture(texture);
        }
        if cfg!(feature = "validation") && !info.usage.contains(TextureUsage::HOST) {
            lock(&self.undefined).push(texture);
        }
    }

    pub fn define(&self, texture: Texture) {
        lock(&self.undefined).retain(|&t| t != texture);
    }

    #[cfg(any(feature = "vk", feature = "mtl"))]
    pub fn acquire(&self, texture: Texture, acquired: bool) {
        if cfg!(feature = "validation") {
            self.presentable
                .get(texture)
                .store(acquired, Ordering::Relaxed);
            if acquired {
                lock(&self.undefined).push(texture);
            }
        }
    }

    pub fn root(&self, texture: Texture) -> Texture {
        let mut root = texture;
        while let Some(&parent) = self.parents.find(root) {
            root = parent;
        }
        root
    }

    pub fn assert_defined(&self, name: &str, texture: Texture) {
        let root = self.root(texture);
        if let Some(acquired) = self.presentable.find(root) {
            check!(
                acquired.load(Ordering::Relaxed),
                "{name} uses the swapchain texture {texture} outside acquire_texture and submit_and_present"
            );
        }
        let undefined = lock(&self.undefined);
        if undefined.is_empty() {
            return;
        }
        check!(
            !undefined.contains(&root),
            "{name} uses {texture} before discard_textures; discard it before its first use"
        );
    }

    pub fn drop_texture(&self, texture: Texture) {
        self.textures.get(texture);
        let parent = self.parents.find(texture).copied();
        if cfg!(feature = "validation") {
            let mut views = lock(&self.views);
            assert!(
                !views.contains(&texture),
                "destroy_texture on {texture} while views of it live; destroy the views first"
            );
            if let Some(i) = parent.and_then(|p| views.iter().position(|&v| v == p)) {
                views.swap_remove(i);
            }
        }
        if parent.is_none() {
            lock(&self.residency).free_texture(texture);
        }
        self.textures.clear(texture);
        self.parents.forget(texture);
        self.presentable.forget(texture);
        self.shared.forget(texture);
        lock(&self.undefined).retain(|&t| t != texture);
    }

    pub fn resolve_view(&self, texture: Texture, info: &TextureViewInfo) -> TextureViewInfo {
        let t = self.textures.get(texture);
        check_ne!(info.format, Format::None, "view format is Format::None");
        check!(
            !info.usage.is_empty() && t.usage.contains(info.usage),
            "view usage {:#x} is not part of {texture} usage {:#x}",
            info.usage,
            t.usage
        );
        check!(
            info.format == t.format || t.usage.contains(TextureUsage::VIEW),
            "a view of {texture} in format {} needs TextureUsage::VIEW",
            info.format
        );
        check_eq!(
            info.format.block(),
            t.format.block(),
            "format {} has a different block than format {}",
            info.format,
            t.format
        );
        let linear = |f: Format| format!("{f:?}").replace("Srgb", "Unorm");
        check!(
            if crate::caps::is_depth_format(t.format) {
                info.format == t.format
            } else if t.format.block()[1] > 1 {
                linear(info.format) == linear(t.format)
            } else {
                !crate::caps::is_depth_format(info.format)
            },
            "a view of {texture} cannot reinterpret {} as {}",
            t.format,
            info.format
        );
        let mips = rest(info.base_mip, info.mips, t.mips, "mip", texture);
        let layers = rest(info.base_layer, info.layers, t.layers, "layer", texture);
        let family = |d: TextureDimension| match d {
            TextureDimension::D1 | TextureDimension::D1Array => 1,
            TextureDimension::D3 => 3,
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
            TextureDimension::Cube | TextureDimension::CubeArray
        );
        match info.dimension {
            TextureDimension::D1 | TextureDimension::D2 | TextureDimension::D3 => {
                check_eq!(
                    layers,
                    1,
                    "a view without layers has one layer, not {layers}"
                )
            }
            TextureDimension::Cube => check!(
                cube && layers == 6,
                "a cube view needs a cube texture and 6 layers"
            ),
            TextureDimension::CubeArray => check!(
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

    pub fn check_view(&self, id: Texture, texture: Texture, info: &TextureViewInfo) {
        let t = *self.textures.get(texture);
        self.parents.set(id, texture);
        if cfg!(feature = "validation") {
            lock(&self.views).push(texture);
        }
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
        check!(
            t.usage.contains(TextureUsage::HOST),
            "write_texture and read_texture need {texture} created with TextureUsage::HOST"
        );
        check!(
            t.samples == 1 && !(Format::D16Unorm..=Format::S8Uint).contains(&t.format),
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
        let [_, bw, bh] = t.format.block();
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
        let pitch = row.div_ceil(bw) as u64 * t.format.block()[0] as u64;
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

    pub fn add_timeline(&self, id: TimelineId) -> Arc<()> {
        let token = Arc::new(());
        lock(&self.timelines).set(id, (0, usize::MAX, Arc::downgrade(&token)));
        token
    }

    pub fn drop_timeline(&self, timeline: &crate::Timeline) {
        self.check_timelines(&[(timeline, 0)]);
        lock(&self.timelines).clear(timeline.id);
    }

    pub fn check_timelines(&self, list: &[(&crate::Timeline, u64)]) {
        if !cfg!(feature = "validation") {
            return;
        }
        let timelines = lock(&self.timelines);
        for &(t, _) in list {
            let (_, _, owner) = timelines.get(t.id);
            check!(
                Weak::ptr_eq(owner, &Arc::downgrade(&t.token)),
                "{} belongs to another device; use the device that created it",
                t.id
            );
        }
    }

    pub fn check_signals(&self, queue: usize, signals: &[(&crate::Timeline, u64)]) {
        if !cfg!(feature = "validation") {
            return;
        }
        let timelines = lock(&self.timelines);
        for (i, &(t, value)) in signals.iter().enumerate() {
            let id = t.id;
            let (last, from, _) = timelines.get(id);
            check!(
                *from != queue || value > *last,
                "submit signals {id} to {value}, but queue {queue} already signalled it to {last}; signal a value above {last}"
            );
            check!(
                signals[..i].iter().all(|s| s.0.id != id),
                "submit signals {id} twice; signal it once with the highest value"
            );
        }
    }

    pub fn record_signals(&self, queue: usize, signals: &[(&crate::Timeline, u64)]) {
        if !cfg!(feature = "validation") {
            return;
        }
        let mut timelines = lock(&self.timelines);
        for &(t, value) in signals {
            let entry = timelines.get_mut(t.id);
            entry.0 = value;
            entry.1 = queue;
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            check!(
                lock(&self.timelines)
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
