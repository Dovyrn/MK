use crate::Error;
use crate::caps::is_depth_format;
use crate::command::Command;
use crate::residency::Residency;
use crate::table::Table;
use crate::table::lock;
use crate::{
    Buffer, BufferUsage, Caps, CommandPool, DescriptorSet, Format, IndexType, IndirectBuffer,
    LoadOp, Memory, QueryKind, QueryPool, ResourceKind, Stage, StoreOp, Texture, TextureDimension,
    TextureInfo, TextureUsage, Topology, sets, slot,
};
use std::sync::atomic::Ordering;

pub struct Check<'a> {
    caps: &'a Caps,
    table: &'a Table,
    pool: CommandPool,
    base: &'static str,
    open: &'static str,
    index: Option<(Buffer, u64, IndexType)>,
    mesh: bool,
    indirect: bool,
    compute: bool,
    render: bool,
    topology: Topology,
    heap: Option<QueryPool>,
    targets: Vec<(Texture, bool)>,
    samples: u32,
    bound: u32,
    labels: Vec<&'static str>,
    needs: u32,
    vertices: u32,
    resolves: Vec<Texture>,
    inputs: [Texture; slot::COLORS as usize],
    reads: u32,
    tile: Option<(u32, u32)>,
    views: u32,
    scissors: u32,
    fresh: u32,
    resident: bool,
    ended: bool,
    layouts: [u32; 3],
    sets: [Option<(DescriptorSet, [u32; 3])>; 3],
    held: Vec<DescriptorSet>,
}

const STATES: [&str; 8] = [
    "set_cull_mode",
    "set_front_face",
    "set_depth_state",
    "set_stencil_reference",
    "set_depth_bias",
    "set_blend_color",
    "set_viewport",
    "set_scissor",
];

fn assert_viewport(name: &str, x: f32, y: f32, width: f32, height: f32, near: f32, far: f32) {
    check!(
        [x, y, width, height].iter().all(|v| v.is_finite()) && width > 0.0 && height > 0.0,
        "{name} {x},{y} {width}x{height} is not a finite positive area; give it a width and height above 0"
    );
    check!(
        (0.0..=1.0).contains(&near) && (0.0..=1.0).contains(&far),
        "{name} depth range {near}..{far} is outside 0..1; keep near and far in 0..1"
    );
}

fn command_name(c: &Command) -> String {
    match c.to_string().split(' ').next().unwrap() {
        "color_attachment" => "begin_render_pass colors".into(),
        "depth_attachment" => "begin_render_pass depth".into(),
        "occlusion_pool" => "begin_render_pass occlusion".into(),
        "tile_memory" => "begin_render_pass tile".into(),
        name => name.into(),
    }
}

fn assert_rects(name: &str, count: usize, max: u32) {
    check!(count > 0, "{name} got an empty list; pass at least one");
    check!(
        count <= max as usize,
        "{name} got {count}, over caps.viewports {max}"
    );
}

fn assert_slot(name: &str, slot: u32, count: u32) {
    check!(slot < count, "{name} slot {slot} is outside 0..{count}");
}

fn assert_span(name: &str, what: &str, base: u32, count: u32, total: u32, texture: Texture) {
    check_ne!(count, 0, "{name} covers 0 {what}s of {texture}");
    let end = if count == !0 {
        total
    } else {
        base.saturating_add(count)
    };
    check!(
        base < total && end <= total,
        "{name} {what}s {base}..{end} are outside the {total} {what}s of {texture}"
    );
}

fn disjoint(a: u32, b: u32, n: u32) -> bool {
    a as u64 + n as u64 <= b as u64 || b as u64 + n as u64 <= a as u64
}

fn copy_block_size(f: Format) -> u32 {
    match f {
        Format::D24UnormS8 | Format::D32FloatS8 => 4,
        f => f.block()[0],
    }
}

impl<'a> Check<'a> {
    pub fn new(
        caps: &'a Caps,
        table: &'a Table,
        pool: CommandPool,
        base: &'static str,
        samples: u32,
    ) -> Self {
        let busy = &table.pools.get(pool).1;
        assert!(
            !busy.swap(true, Ordering::Acquire),
            "begin_commands on {pool}, which is already recording a command buffer; end or drop that one first"
        );
        Check {
            caps,
            table,
            pool,
            base,
            open: base,
            index: None,
            mesh: false,
            indirect: false,
            compute: false,
            render: false,
            topology: Topology::Triangle,
            heap: None,
            targets: Vec::new(),
            samples,
            bound: 0,
            labels: Vec::new(),
            needs: 0,
            vertices: 0,
            resolves: Vec::new(),
            inputs: [Texture::default(); slot::COLORS as usize],
            reads: 0,
            tile: None,
            views: 0,
            scissors: 0,
            fresh: 0,
            resident: base != "outside",
            ended: false,
            layouts: [0; 3],
            sets: [None; 3],
            held: Vec::new(),
        }
    }

    pub fn queue(&self) -> usize {
        self.table.pools.get(self.pool).0
    }

    fn graphics(&self) -> bool {
        self.caps.graphics_queues >> self.queue() & 1 != 0
    }

    pub fn finish(&self) {
        check_eq!(
            self.open,
            self.base,
            "the command buffer ends inside a {} pass; call end() on the pass first",
            self.open
        );
        check!(
            self.labels.is_empty(),
            "the command buffer ends with {} debug labels open; call end_debug_label for each",
            self.labels.len()
        );
    }

    pub fn end(&mut self) {
        self.finish();
        self.ended = true;
        self.table
            .pools
            .get(self.pool)
            .1
            .store(false, Ordering::Release);
    }

    pub fn assert_ended(&self) {
        check!(
            self.ended,
            "submit needs command buffers ended with end(); call end() on each first"
        );
    }

    fn require(&self, c: &Command, has: bool, cap: &str) -> Result<(), Error> {
        if has {
            Ok(())
        } else {
            Err(Error::Unsupported(format!(
                "{} needs caps.{cap}",
                command_name(c)
            )))
        }
    }

    fn assert_resident(&self, name: &str, what: &dyn std::fmt::Display, count: u32) {
        if Residency::is_exempt(count) {
            return;
        }
        check!(
            count > 0,
            "{name} uses {what}, which no committed residency set holds; add_residency it and commit_residency"
        );
        check!(
            self.resident,
            "{name} uses {what} before use_residency_set in this command buffer"
        );
    }

    fn buffer_range(&self, name: &str, buffer: Buffer, offset: u64, size: u64) -> u64 {
        let total = *self.table.buffers.get(buffer);
        self.assert_resident(name, &buffer, lock(&self.table.residency).buffer(buffer));
        let end = if size == !0 {
            total
        } else {
            offset.saturating_add(size)
        };
        check!(
            offset <= total && end <= total,
            "{name} reads bytes {offset}..{end} of {buffer}, which has {total}"
        );
        end - offset
    }

    fn limited_range(
        &self,
        c: &Command,
        buffer: Buffer,
        offset: u64,
        size: u64,
        max: u64,
    ) -> Result<(), Error> {
        let size = self.buffer_range(&command_name(c), buffer, offset, size);
        check!(size > 0, "{} covers 0 bytes of {buffer}", command_name(c));
        if size > max {
            return Err(Error::Unsupported(format!(
                "{} binds {size} bytes, over the limit of {max}",
                command_name(c)
            )));
        }
        Ok(())
    }

    fn check_wait(&self, name: &str, src: Stage, dst: Stage) {
        check_eq!(
            src.is_empty(),
            dst.is_empty(),
            "{name} waits with src_stage {src:?} and dst_stage {dst:?}; give both or neither"
        );
        check!(
            !src.contains(Stage::HOST),
            "{name} waits on Stage::HOST; host writes before submit need no wait"
        );
    }

    fn texture_with_usage(
        &self,
        name: &str,
        texture: Texture,
        needs: TextureUsage,
        what: &str,
    ) -> &TextureInfo<'static> {
        let t = self.table.textures.get(texture);
        let copies = if t.usage.contains(TextureUsage::HOST) {
            TextureUsage::COPY_SRC | TextureUsage::COPY_DST
        } else {
            TextureUsage::default()
        };
        check!(
            !((t.usage | copies) & needs).is_empty(),
            "{name} needs {texture} created with TextureUsage::{what}"
        );
        self.table.assert_defined(name, texture);
        let root = self.table.root(texture);
        self.assert_resident(name, &texture, lock(&self.table.residency).texture(root));
        t
    }

    fn texture_region(
        &self,
        name: &str,
        texture: Texture,
        (mip, layer, layers): (u32, u32, u32),
        at: [u32; 3],
        size: [u32; 3],
    ) -> &TextureInfo<'static> {
        let t = self.table.textures.get(texture);
        assert_span(name, "mip", mip, 1, t.mips, texture);
        assert_span(name, "layer", layer, layers, t.layers, texture);
        let end = [t.width, t.height, t.depth].map(|n| (n >> mip).max(1));
        check!(
            (0..3).all(|i| at[i] as u64 + size[i] as u64 <= end[i] as u64),
            "{name} region {at:?} {size:?} is outside mip {mip} of {texture} ({end:?})"
        );
        let [_, bw, bh] = t.format.block();
        let edge = |i: usize, b: u32| size[i].is_multiple_of(b) || at[i] + size[i] == end[i];
        check!(
            at[0].is_multiple_of(bw) && at[1].is_multiple_of(bh) && edge(0, bw) && edge(1, bh),
            "{name} region {at:?} {size:?} is not aligned to the {bw}x{bh} blocks of {texture}"
        );
        t
    }

    fn query_range(
        &self,
        name: &str,
        pool: QueryPool,
        kinds: &[QueryKind],
        first: u32,
        count: u32,
    ) {
        let &(kind, total) = self.table.queries.get(pool);
        self.assert_resident(name, &pool, lock(&self.table.residency).query(pool));
        check!(
            kinds.is_empty() || kinds.contains(&kind),
            "{name} needs a {kinds:?} pool, but {pool} is {kind}"
        );
        if count > 0 {
            let last = first.saturating_add(count - 1);
            check!(
                last < total,
                "{name} query {last} is outside the {total} queries of {pool}"
            );
        }
    }

    fn mark_state(&mut self, state: u32) {
        self.fresh |= 1 << state;
    }

    fn switch(&mut self, ids: [u32; 3]) {
        for k in 0..3 {
            if self.sets[k].is_some_and(|(_, at)| at[..=k] != ids[..=k]) {
                self.sets[k] = None;
            }
        }
        self.layouts = ids;
    }

    fn check_sets(&self, name: &str, ids: [u32; 3]) {
        for (k, &id) in ids.iter().enumerate().filter(|&(_, &id)| id != 0) {
            let bound = self.sets[k].map(|(set, _)| self.table.sets.get(set).layout);
            check!(
                bound.is_some_and(|l| self.table.layouts.get(l).id == id),
                "{name} needs bind_descriptor_set for set {} of the pipeline; binding a pipeline whose lower sets differ unbinds the higher ones",
                k as u32 + sets::BUFFERS
            );
        }
    }

    fn bind_set(&mut self, name: &str, set: DescriptorSet, offsets: Option<&[u32]>) {
        check!(
            self.compute || self.render || self.tile.is_some(),
            "{name} needs a pipeline bound earlier in this pass"
        );
        let e = self.table.sets.get(set);
        let layout = self.table.layouts.get(e.layout);
        let k = (layout.set - sets::BUFFERS) as usize;
        if self.compute || self.render {
            check_eq!(
                self.layouts[k],
                layout.id,
                "{name} {set} has a different layout than set {} of the bound pipeline",
                layout.set
            );
        }
        let offsets = match offsets {
            Some(list) => {
                check!(
                    !layout.dynamic.is_empty(),
                    "{name} on {set}, which has no dynamic descriptors; use bind_descriptor_set"
                );
                list
            }
            None => {
                check!(
                    layout.dynamic.is_empty(),
                    "{name} on {set}, which has dynamic descriptors; use bind_descriptor_set_dynamic with their offsets"
                );
                &[]
            }
        };
        check_eq!(
            offsets.len(),
            layout.dynamic.len(),
            "{name} got {} offsets for {} dynamic descriptors in {set}; pass one per dynamic descriptor, by binding then element",
            offsets.len(),
            layout.dynamic.len()
        );
        let alignment = self.caps.bind_alignment.max(1);
        let resources = lock(&e.resources);
        for (&offset, &(binding, element)) in offsets.iter().zip(&layout.dynamic) {
            check!(
                offset.is_multiple_of(alignment),
                "{name} offset {offset} for binding {binding} is not a multiple of caps.bind_alignment {alignment}"
            );
            let found = resources.iter().find(|r| (r.0, r.1) == (binding, element));
            check!(
                found.is_some(),
                "{name} {set} binding {binding} element {element} was never written"
            );
            let &(_, _, r, end) = found.unwrap();
            let buffer = Buffer::from_raw(r.handle);
            let total = *self.table.buffers.get(buffer);
            let moved = end + offset as u64;
            check!(
                moved <= total,
                "{name} offset {offset} moves binding {binding} to end at byte {moved} of {buffer}, which has {total}"
            );
        }
        for &(_, _, r, _) in resources.iter() {
            match r.kind {
                ResourceKind::Buffer => {
                    let b = Buffer::from_raw(r.handle);
                    self.assert_resident(name, &b, lock(&self.table.residency).buffer(b));
                }
                _ => {
                    let t = Texture::from_raw(r.handle);
                    let root = self.table.root(t);
                    self.assert_resident(name, &t, lock(&self.table.residency).texture(root));
                }
            }
        }
        self.sets[k] = Some((set, self.layouts));
        if !self.held.contains(&set) {
            e.bound.fetch_add(1, Ordering::Relaxed);
            self.held.push(set);
        }
    }

    fn check_draw(&self, c: &Command, indexed: bool) {
        check!(
            self.render,
            "{} needs bind_render_pipeline earlier in this pass",
            command_name(c)
        );
        self.check_sets(&command_name(c), self.layouts);
        if !matches!(
            c,
            Command::ExecuteIndirect { .. } | Command::ExecuteIndirectCount { .. }
        ) {
            let mesh = matches!(
                c,
                Command::DrawMeshTasks { .. } | Command::DrawMeshTasksIndirect { .. }
            );
            check_eq!(
                self.mesh,
                mesh,
                "{} needs a {} pipeline bound",
                command_name(c),
                if mesh { "mesh" } else { "vertex" }
            );
        }
        check_eq!(
            self.views,
            self.scissors,
            "{} has {} viewports but {} scissors; set one scissor per viewport",
            command_name(c),
            self.views,
            self.scissors
        );
        let missing = !self.fresh & ((1 << STATES.len()) - 1);
        if missing != 0 {
            let state = STATES[missing.trailing_zeros() as usize];
            panic!(
                "{} needs {state} earlier in this pass; a render pass starts with no state",
                command_name(c)
            );
        }
        check!(
            self.samples == 0 || self.bound == self.samples,
            "{} draws with a {}-sample pipeline in a {}-sample pass; create the pipeline with the pass's sample count",
            command_name(c),
            self.bound,
            self.samples
        );
        let missing = self.needs & !self.vertices;
        check_eq!(
            missing,
            0,
            "{} needs bind_vertex_buffer for slots {missing:#b}, which the pipeline's attributes read",
            command_name(c)
        );
        check!(
            !indexed || self.index.is_some(),
            "{} needs bind_index_buffer earlier in this pass",
            command_name(c)
        );
        for i in (0..slot::COLORS).filter(|i| self.reads & 1 << i != 0) {
            let texture = self.inputs[i as usize];
            check!(
                !texture.is_null(),
                "{} reads attachment {i}; give the pass a TextureUsage::INPUT color there",
                command_name(c)
            );
        }
    }

    fn indirect_list(&self, c: &Command, indirect: IndirectBuffer) -> Result<u32, Error> {
        let name = &command_name(c);
        let e = *self.table.indirects.get(indirect);
        self.assert_resident(
            name,
            &indirect,
            lock(&self.table.residency).indirect(indirect),
        );
        self.check_draw(c, e.info.indexed);
        check!(
            self.indirect,
            "{name} needs a pipeline created with indirect: true"
        );
        check_eq!(
            self.mesh,
            e.mesh,
            "{name} needs a mesh pipeline exactly when {indirect} was made for one"
        );
        check_eq!(
            self.topology,
            e.topology,
            "{name} draws with {} but {indirect} holds {}",
            self.topology,
            e.topology
        );
        if e.info.indexed {
            let (buffer, offset, kind) = (e.info.indices, e.info.offset, e.info.kind);
            check_eq!(
                self.index,
                Some((buffer, offset, kind)),
                "{name} needs bind_index_buffer of {buffer} at offset {offset} with {kind}"
            );
            if offset != 0 {
                self.require(c, self.caps.indirect_offset, "indirect_offset")?;
            }
        }
        if e.mesh {
            self.require(c, self.caps.mesh_indirect, "mesh_indirect")?;
        }
        Ok(e.info.draws)
    }

    fn check_transient(&self, name: &str, texture: Texture, load: LoadOp, store: StoreOp) {
        if self.table.textures.get(texture).memory == Memory::Transient {
            check!(
                load != LoadOp::Load && store == StoreOp::DontCare,
                "{name} loads or stores Memory::Transient {texture}; use LoadOp::Clear or LoadOp::DontCare and StoreOp::DontCare"
            );
        }
    }

    fn check_indirect_buffer(&self, name: &str, buffer: Buffer, offset: u64, size: u64) {
        check!(
            offset.is_multiple_of(4),
            "{name} offset {offset} is not a multiple of 4"
        );
        self.table
            .assert_usage(name, buffer, BufferUsage::INDIRECT, "INDIRECT");
        self.buffer_range(name, buffer, offset, size);
    }

    fn check_buffer_copy(&self, c: &Command) {
        let (Command::CopyBufferToTexture {
            buffer,
            offset,
            row_length,
            row_count,
            texture,
            mip,
            layer,
            x,
            y,
            z,
            width: w,
            height: h,
            depth: d,
        }
        | Command::CopyTextureToBuffer {
            buffer,
            offset,
            row_length,
            row_count,
            texture,
            mip,
            layer,
            x,
            y,
            z,
            width: w,
            height: h,
            depth: d,
        }) = *c
        else {
            return;
        };
        let upload = matches!(c, Command::CopyBufferToTexture { .. });
        let (needs, what) = if upload {
            (TextureUsage::COPY_DST, "COPY_DST")
        } else {
            (TextureUsage::COPY_SRC, "COPY_SRC")
        };
        let name = command_name(c);
        let t = self.texture_with_usage(&name, texture, needs, what);
        let (needs, what) = if upload {
            (BufferUsage::COPY_SRC, "COPY_SRC")
        } else {
            (BufferUsage::COPY_DST, "COPY_DST")
        };
        self.table.assert_usage(&name, buffer, needs, what);
        check!(
            self.graphics() || !is_depth_format(t.format),
            "{name} on depth format {} needs a graphics queue",
            t.format
        );
        check!(
            w > 0 && h > 0 && d > 0,
            "{} region {w}x{h}x{d} has a zero",
            command_name(c)
        );
        assert_span(&name, "mip", mip, 1, t.mips, texture);
        assert_span(&name, "layer", layer, 1, t.layers, texture);
        let size = [
            (t.width >> mip).max(1),
            (t.height >> mip).max(1),
            (t.depth >> mip).max(1),
        ];
        let fits = |at: u32, n: u32, end: u32| at as u64 + n as u64 <= end as u64;
        check!(
            fits(x, w, size[0]) && fits(y, h, size[1]) && fits(z, d, size[2]),
            "{name} region {x},{y},{z} {w}x{h}x{d} is outside mip {mip} of {texture} ({}x{}x{})",
            size[0],
            size[1],
            size[2]
        );
        let [_, bw, bh] = t.format.block();
        let edge = |at: u32, n: u32, b: u32, end: u32| {
            at.is_multiple_of(b) && (n.is_multiple_of(b) || at + n == end)
        };
        check!(
            edge(x, w, bw, size[0]) && edge(y, h, bh, size[1]),
            "{} region {x},{y} {w}x{h} is not aligned to the {bw}x{bh} blocks of {texture}",
            command_name(c)
        );
        check!(
            row_length.is_multiple_of(bw) && row_count.is_multiple_of(bh),
            "{} rows {row_length}x{row_count} are not multiples of the {bw}x{bh} blocks",
            command_name(c)
        );
        let texel = copy_block_size(t.format).max(4) as u64;
        check!(
            offset.is_multiple_of(texel),
            "{} buffer offset {offset} is not a multiple of {texel}",
            command_name(c)
        );
        let row = if row_length == 0 { w } else { row_length };
        let rows = if row_count == 0 { h } else { row_count };
        check!(
            row >= w && rows >= h,
            "{} rows {row}x{rows} are smaller than the region {w}x{h}",
            command_name(c)
        );
        let bytes = row.div_ceil(bw) as u64 * copy_block_size(t.format) as u64;
        let total = bytes
            .checked_mul(rows.div_ceil(bh) as u64)
            .and_then(|n| n.checked_mul(d as u64))
            .unwrap_or_else(|| panic!("{} region size overflows", command_name(c)));
        self.buffer_range(&name, buffer, offset, total);
    }

    pub fn clear_pass(&mut self) {
        self.open = "outside";
        self.targets.clear();
        self.resolves.clear();
        self.heap = None;
        self.inputs = [Texture::default(); slot::COLORS as usize];
        self.tile = None;
    }

    pub fn step(&mut self, c: &Command) -> Result<(), Error> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let scope = c.scope();
        let open = self.open;
        let name = &command_name(c);
        check!(
            !self.ended,
            "{name} after end(); record into a new command buffer"
        );
        check!(
            scope.is_empty() || scope.split(' ').any(|s| s == open),
            "{name} runs only in {scope}, not {open}"
        );
        let caps = self.caps;
        match *c {
            Command::BeginRenderPass {
                width,
                height,
                layers,
                src_stage,
                dst_stage,
            } => {
                self.check_wait(name, src_stage, dst_stage);
                check!(self.graphics(), "begin_render_pass needs a graphics queue");
                check!(
                    width > 0 && height > 0 && layers > 0,
                    "begin_render_pass size {width}x{height}x{layers} has a zero"
                );
                let samples: Vec<u32> = self
                    .targets
                    .iter()
                    .map(|&(t, _)| self.table.textures.get(t).samples)
                    .collect();
                check!(
                    samples.windows(2).all(|w| w[0] == w[1]),
                    "begin_render_pass attachments have different sample counts {samples:?}; give them one count"
                );
                self.samples = samples.first().copied().unwrap_or(0);
                for &(texture, clear) in &self.targets {
                    let t = self.table.textures.get(texture);
                    check!(
                        width <= t.width && height <= t.height && layers <= t.layers,
                        "begin_render_pass {width}x{height}x{layers} is larger than {texture} ({}x{}x{})",
                        t.width,
                        t.height,
                        t.layers
                    );
                    check!(
                        !clear || (width == t.width && height == t.height),
                        "begin_render_pass {width}x{height} clears {texture} ({}x{}) but does not cover it; match its size or use LoadOp::Load",
                        t.width,
                        t.height
                    );
                }
                for &r in &self.resolves {
                    let t = self.table.textures.get(r);
                    if width < t.width || height < t.height {
                        self.require(c, caps.partial_resolve, "partial_resolve")?;
                    }
                }
                self.open = "render";
                self.render = false;
                self.layouts = [0; 3];
                self.sets = [None; 3];
                self.views = 0;
                self.scissors = 0;
                self.fresh = 0;
            }
            Command::BeginComputePass {
                src_stage,
                dst_stage,
            } => {
                self.check_wait(name, src_stage, dst_stage);
                self.open = "compute";
                self.compute = false;
                self.layouts = [0; 3];
                self.sets = [None; 3];
            }
            Command::EndComputePass => {
                check_ne!(
                    self.labels.last(),
                    Some(&"compute"),
                    "the compute pass ends with a debug label open; call end_debug_label first"
                );
                self.open = "outside";
            }
            Command::EndRenderPass => {
                check_ne!(
                    self.labels.last(),
                    Some(&"render"),
                    "the render pass ends with a debug label open; call end_debug_label first"
                );
                self.clear_pass();
            }
            Command::BindComputePipeline { pipeline } => {
                self.require(c, caps.compute, "compute")?;
                let ids = *self.table.pipelines.get(pipeline);
                self.switch(ids);
                self.compute = true;
            }
            Command::Dispatch { .. } => {
                self.require(c, caps.compute, "compute")?;
                check!(
                    self.compute,
                    "dispatch needs bind_compute_pipeline earlier in this pass"
                );
                self.check_sets(name, self.layouts);
            }
            Command::DispatchIndirect { buffer, offset } => {
                self.require(c, caps.compute, "compute")?;
                check!(
                    self.compute,
                    "dispatch_indirect needs bind_compute_pipeline earlier in this pass"
                );
                self.check_sets(name, self.layouts);
                check!(
                    offset.is_multiple_of(4),
                    "dispatch_indirect offset {offset} is not a multiple of 4"
                );
                self.table
                    .assert_usage(name, buffer, BufferUsage::INDIRECT, "INDIRECT");
                self.buffer_range(name, buffer, offset, 12);
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => {
                check!(size > 0, "copy_buffer size is 0");
                self.table
                    .assert_usage(name, src, BufferUsage::COPY_SRC, "COPY_SRC");
                self.table
                    .assert_usage(name, dst, BufferUsage::COPY_DST, "COPY_DST");
                let len = self.buffer_range(name, src, src_offset, size);
                self.buffer_range(name, dst, dst_offset, size);
                check!(
                    src != dst || src_offset + len <= dst_offset || dst_offset + len <= src_offset,
                    "copy_buffer source and destination overlap in {src}"
                );
            }
            Command::FillBuffer {
                buffer,
                offset,
                size,
                ..
            } => {
                self.require(c, caps.buffer_clear, "buffer_clear")?;
                check!(
                    offset.is_multiple_of(4) && (size == !0 || size.is_multiple_of(4)),
                    "fill_buffer offset {offset} and size {size} must be multiples of 4"
                );
                self.table
                    .assert_usage(name, buffer, BufferUsage::COPY_DST, "COPY_DST");
                self.limited_range(c, buffer, offset, size, !0)?;
            }
            Command::MemoryBarrier {
                src_stage,
                dst_stage,
            } => {
                check!(
                    !src_stage.is_empty() && !dst_stage.is_empty(),
                    "{name} needs a src_stage and a dst_stage"
                );
                let pass = Stage::COMPUTE | Stage::COPY;
                check!(
                    (src_stage & !pass).is_empty()
                        && (dst_stage & !(pass | Stage::INDIRECT | Stage::HOST)).is_empty(),
                    "{name} {src_stage:?} -> {dst_stage:?} orders work outside this compute pass; wait on it with src_stage and dst_stage at the next pass's begin"
                );
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::PresentBarrier { texture, src_stage } => {
                check!(!src_stage.is_empty(), "{name} needs a src_stage");
                check!(
                    self.table.presentable.find(texture).is_some(),
                    "{name} on {texture}, which is not a swapchain texture"
                );
                self.table.assert_defined(name, texture);
            }
            #[cfg(feature = "vk")]
            Command::AcquireShared {
                texture,
                dst_stage: stage,
            }
            | Command::ReleaseShared {
                texture,
                src_stage: stage,
            } => {
                check!(!stage.is_empty(), "{name} needs a stage");
                self.table.textures.get(texture);
                check!(
                    self.table.shared.find(texture).is_some(),
                    "{name} on {texture}, which import_shared_texture did not make"
                );
            }
            Command::DiscardTextures { textures } => {
                check!(!textures.is_empty(), "{name} has no textures");
                for d in textures {
                    check!(!d.dst_stage.is_empty(), "{name} needs a dst_stage");
                    if let Some(acquired) = self.table.presentable.find(d.texture) {
                        check!(
                            acquired.load(Ordering::Relaxed),
                            "{name} on the swapchain texture {} outside acquire_texture and submit_and_present",
                            d.texture
                        );
                        check!(
                            !d.src_stage.is_empty(),
                            "{name} on the swapchain texture {} needs a src_stage, or its layout change can run before the acquire",
                            d.texture
                        );
                    }
                    self.table.textures.get(d.texture);
                    check!(
                        self.table.parents.find(d.texture).is_none(),
                        "{name} discards the view {}; discard the texture it views",
                        d.texture
                    );
                    self.table.define(d.texture);
                }
            }
            Command::CopyBufferToTexture { .. } | Command::CopyTextureToBuffer { .. } => {
                self.check_buffer_copy(c);
            }
            Command::CopyTexture {
                src,
                src_mip,
                src_layer,
                src_x,
                src_y,
                src_z,
                dst,
                dst_mip,
                dst_layer,
                dst_x,
                dst_y,
                dst_z,
                width,
                height,
                depth: d,
                layers,
            } => {
                self.texture_with_usage(name, src, TextureUsage::COPY_SRC, "COPY_SRC");
                self.texture_with_usage(name, dst, TextureUsage::COPY_DST, "COPY_DST");
                let size = [width, height, d];
                check!(
                    size.iter().all(|&n| n > 0),
                    "copy_texture region {width}x{height}x{d} has a zero"
                );
                let from = (src_mip, src_layer, layers);
                let s = self.texture_region(name, src, from, [src_x, src_y, src_z], size);
                let to = (dst_mip, dst_layer, layers);
                let t = self.texture_region(name, dst, to, [dst_x, dst_y, dst_z], size);
                check!(
                    self.graphics() || !is_depth_format(s.format) && !is_depth_format(t.format),
                    "copy_texture on depth formats needs a graphics queue"
                );
                check_eq!(
                    s.format.block(),
                    t.format.block(),
                    "copy_texture formats {} and {} have different blocks",
                    s.format,
                    t.format
                );
                check!(
                    s.format == t.format
                        || !is_depth_format(s.format) && !is_depth_format(t.format),
                    "copy_texture between depth formats {} and {} needs one format",
                    s.format,
                    t.format
                );
                check_eq!(
                    s.samples,
                    t.samples,
                    "copy_texture sample counts {} and {} differ",
                    s.samples,
                    t.samples
                );
                check!(
                    src != dst
                        || src_mip != dst_mip
                        || disjoint(src_layer, dst_layer, layers)
                        || disjoint(src_x, dst_x, width)
                        || disjoint(src_y, dst_y, height)
                        || disjoint(src_z, dst_z, d),
                    "copy_texture source and destination overlap in {src}"
                );
                let line = |t: &TextureInfo| {
                    matches!(
                        t.dimension,
                        TextureDimension::D1 | TextureDimension::D1Array
                    )
                };
                check_eq!(
                    line(s),
                    line(t),
                    "copy_texture cannot copy between a 1D texture and a 2D or 3D one"
                );
                check_eq!(
                    s.format,
                    t.format,
                    "copy_texture from format {} to {}; to reinterpret, copy through a texture view in the other format",
                    s.format,
                    t.format
                );
            }
            Command::ColorAttachment {
                index,
                texture,
                load: ld,
                store: st,
                resolve,
                ..
            } => {
                assert_slot(name, index, slot::COLORS);
                let t = self.texture_with_usage(name, texture, TextureUsage::COLOR, "COLOR");
                if t.usage.contains(TextureUsage::INPUT) {
                    self.inputs[index as usize] = texture;
                }
                self.check_transient(name, texture, ld, st);
                self.targets.push((texture, ld == LoadOp::Clear));
                if let Some(resolve) = resolve {
                    self.resolves.push(resolve);
                    let r = self.texture_with_usage(name, resolve, TextureUsage::COLOR, "COLOR");
                    let s = self.table.textures.get(texture);
                    check!(
                        s.samples > 1 && r.samples == 1,
                        "{name} resolve needs multisampled {texture} and single-sample {resolve}"
                    );
                    check!(
                        s.format == r.format && s.width == r.width && s.height == r.height,
                        "{name} resolve {resolve} differs in format or size from {texture}"
                    );
                    check!(
                        !s.format.uint() && !s.format.sint(),
                        "{name} resolves integer format {}, which no backend can average",
                        s.format
                    );
                }
            }
            Command::DepthAttachment {
                texture,
                load: ld,
                store: st,
                clear_depth: depth,
                stencil_load,
                stencil_store,
                ..
            } => {
                check!(
                    (0.0..=1.0).contains(&depth),
                    "{name} clear depth {depth} is outside 0..1"
                );
                self.texture_with_usage(name, texture, TextureUsage::DEPTH, "DEPTH");
                self.check_transient(name, texture, ld, st);
                self.check_transient(name, texture, stencil_load, stencil_store);
                let clear = ld == LoadOp::Clear || stencil_load == LoadOp::Clear;
                self.targets.push((texture, clear));
            }
            Command::BindRenderPipeline { pipeline } => {
                let entry = *self.table.renders.get(pipeline);
                let samples = entry.samples;
                self.reads = entry.reads;
                check!(
                    self.samples == 0 || samples == self.samples,
                    "bind_render_pipeline {pipeline} has {samples} samples but the pass has {}",
                    self.samples
                );
                self.switch(entry.layouts);
                self.render = true;
                self.bound = samples;
                self.needs = entry.vertices;
                self.topology = entry.topology;
                self.mesh = entry.mesh;
                self.indirect = entry.indirect;
            }
            Command::InputAttachmentBarrier => {
                self.require(c, caps.attachment_reads, "attachment_reads")?;
            }
            Command::TileMemory {
                width,
                height,
                shared,
            } => {
                self.require(c, caps.tiles, "tiles")?;
                let sizes = [8, 16, 32];
                check!(
                    sizes.contains(&width) && sizes.contains(&height),
                    "{name} size {width}x{height} is not 8, 16 or 32 on each side"
                );
                check!(
                    shared.is_multiple_of(16),
                    "{name} shared memory {shared} is not a multiple of 16"
                );
                if shared > caps.max_shared {
                    return Err(Error::Unsupported(format!(
                        "{name} shared memory {shared} is over caps.max_shared {}",
                        caps.max_shared
                    )));
                }
                self.tile = Some((width, height));
            }
            Command::DispatchTile { pipeline } => {
                self.require(c, caps.tiles, "tiles")?;
                let (samples, ids) = *self.table.tile_pipelines.get(pipeline);
                self.check_sets(name, ids);
                check!(
                    self.tile.is_some(),
                    "dispatch_tile needs RenderPassInfo.tile set in begin_render_pass"
                );
                check!(
                    self.samples == 0 || samples == self.samples,
                    "dispatch_tile {pipeline} has {samples} samples but the pass has {}",
                    self.samples
                );
                self.render = false;
            }
            Command::SetCullMode { .. } => self.mark_state(0),
            Command::SetFrontFace { .. } => self.mark_state(1),
            Command::SetStencilReference { .. } => self.mark_state(3),
            Command::SetBlendColor { .. } => self.mark_state(5),
            Command::SetPrimitiveTopology { topology } => self.topology = topology,
            Command::Draw { .. } => self.check_draw(c, false),
            Command::DrawIndexed { .. } => self.check_draw(c, true),
            Command::SetDepthState { state } => {
                self.table.states.get(state);
                self.mark_state(2);
            }
            Command::SetDepthBias { clamp, .. } => {
                self.mark_state(4);
                if clamp != 0.0 {
                    self.require(c, caps.bias_clamp, "bias_clamp")?;
                }
            }
            Command::UseResidencySet { set } => {
                lock(&self.table.residency).assert_set(set);
                self.resident = true;
            }
            Command::BindDescriptorSet { set } => self.bind_set(name, set, None),
            Command::BindDescriptorSetDynamic { set, offsets } => {
                self.bind_set(name, set, Some(offsets))
            }
            Command::BindVertexBuffer {
                slot: i,
                buffer,
                offset,
            } => {
                assert_slot(name, i, slot::VERTICES);
                self.table
                    .assert_usage(name, buffer, BufferUsage::VERTEX, "VERTEX");
                self.buffer_range(name, buffer, offset, 0);
                self.vertices |= 1 << i;
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => {
                let size = if kind == IndexType::U16 { 2 } else { 4 };
                check!(
                    offset.is_multiple_of(size),
                    "bind_index_buffer offset {offset} is not a multiple of {size}"
                );
                self.table
                    .assert_usage(name, buffer, BufferUsage::INDEX, "INDEX");
                self.buffer_range(name, buffer, offset, 0);
                self.index = Some((buffer, offset, kind));
            }
            Command::DrawIndirect { buffer, offset } => {
                self.require(c, caps.indirect, "indirect")?;
                self.check_draw(c, false);
                self.check_indirect_buffer(name, buffer, offset, 16);
            }
            Command::DrawIndexedIndirect { buffer, offset } => {
                self.require(c, caps.indirect, "indirect")?;
                self.check_draw(c, true);
                if self.index.map(|i| i.1) != Some(0) {
                    self.require(c, caps.indirect_offset, "indirect_offset")?;
                }
                self.check_indirect_buffer(name, buffer, offset, 20);
            }
            Command::ExecuteIndirect {
                indirect,
                first,
                count,
            } => {
                let draws = self.indirect_list(c, indirect)?;
                if count > 1 {
                    self.require(c, caps.multi_draw, "multi_draw")?;
                }
                check!(
                    first as u64 + count as u64 <= draws as u64,
                    "execute_indirect draws {first}..{} past the {draws} draws of {indirect}",
                    first as u64 + count as u64
                );
            }
            Command::ExecuteIndirectCount {
                indirect,
                buffer,
                offset,
            } => {
                if self.indirect_list(c, indirect)? > 1 {
                    self.require(c, caps.multi_draw, "multi_draw")?;
                }
                check!(
                    offset.is_multiple_of(4),
                    "execute_indirect_count offset {offset} is not a multiple of 4"
                );
                self.table
                    .assert_usage(name, buffer, BufferUsage::INDIRECT, "INDIRECT");
                self.buffer_range(name, buffer, offset, 8);
                self.require(c, caps.indirect_count, "indirect_count")?;
            }
            Command::DrawMeshTasks { x, y, z } => {
                self.require(c, caps.mesh, "mesh")?;
                self.check_draw(c, false);
                let total = x as u64 * y as u64 * z as u64;
                if x.max(y).max(z) > caps.mesh_axis || total > caps.mesh_groups as u64 {
                    return Err(Error::Unsupported(format!(
                        "{name} grid {x}x{y}x{z} is over caps.mesh_axis {} or caps.mesh_groups {}",
                        caps.mesh_axis, caps.mesh_groups
                    )));
                }
            }
            Command::DrawMeshTasksIndirect { buffer, offset } => {
                self.require(c, caps.mesh_indirect, "mesh_indirect")?;
                self.check_draw(c, false);
                self.check_indirect_buffer(name, buffer, offset, 12);
            }
            Command::OcclusionPool { pool } => {
                let kinds = [QueryKind::Occlusion, QueryKind::Precise];
                self.query_range(name, pool, &kinds, 0, 0);
                self.heap = Some(pool);
            }
            Command::ResetQueries { pool, first, count } => {
                self.query_range(name, pool, &[], first, count);
            }
            Command::WriteTimestamp { pool, index, stage } => {
                check_eq!(
                    stage.0.count_ones(),
                    1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
                self.require(c, caps.timestamps, "timestamps")?;
                self.query_range(name, pool, &[QueryKind::Timestamp], index, 1);
            }
            Command::BeginOcclusionQuery { pool, index }
            | Command::EndOcclusionQuery { pool, index } => {
                self.require(c, caps.occlusion, "occlusion")?;
                check_eq!(
                    self.heap,
                    Some(pool),
                    "{name} needs RenderPassInfo.occlusion set to {pool} in begin_render_pass"
                );
                let kinds = [QueryKind::Occlusion, QueryKind::Precise];
                self.query_range(name, pool, &kinds, index, 1);
            }
            Command::ResolveTimestamps {
                pool,
                first,
                count,
                buffer,
                offset,
            }
            | Command::ResolveOcclusion {
                pool,
                first,
                count,
                buffer,
                offset,
            } => {
                let kinds: &[QueryKind] = if matches!(c, Command::ResolveTimestamps { .. }) {
                    &[QueryKind::Timestamp]
                } else {
                    &[QueryKind::Occlusion, QueryKind::Precise]
                };
                self.query_range(name, pool, kinds, first, count);
                check!(
                    offset.is_multiple_of(8),
                    "{name} offset {offset} is not a multiple of 8"
                );
                self.table
                    .assert_usage(name, buffer, BufferUsage::COPY_DST, "COPY_DST");
                self.buffer_range(name, buffer, offset, count as u64 * 8);
            }
            Command::BeginDebugLabel { .. } => {
                self.labels.push(open);
            }
            Command::EndDebugLabel => {
                check_eq!(
                    self.labels.pop(),
                    Some(open),
                    "end_debug_label has no begin_debug_label open in this pass"
                );
            }
            Command::SetViewport { viewports } => {
                assert_rects(name, viewports.len(), caps.viewports);
                for v in viewports {
                    assert_viewport(name, v.x, v.y, v.width, v.height, v.near, v.far);
                }
                self.views = viewports.len() as u32;
                self.mark_state(6);
            }
            Command::SetScissor { scissors } => {
                assert_rects(name, scissors.len(), caps.viewports);
                for r in scissors {
                    let end = (r.x as u64 + r.width as u64).max(r.y as u64 + r.height as u64);
                    check!(
                        end <= i32::MAX as u64,
                        "set_scissor {r:?} ends past i32::MAX"
                    );
                }
                self.scissors = scissors.len() as u32;
                self.mark_state(7);
            }
        }
        Ok(())
    }
}

impl Drop for Check<'_> {
    fn drop(&mut self) {
        for &set in &self.held {
            if let Some(e) = self.table.sets.find(set) {
                e.bound.fetch_sub(1, Ordering::Relaxed);
            }
        }
        if !self.ended
            && let Some(pool) = self.table.pools.find(self.pool)
        {
            pool.1.store(false, Ordering::Release);
        }
    }
}
