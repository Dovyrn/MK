use crate::caps::is_depth_format;
use crate::command::Command;
use crate::residency::Residency;
use crate::table::Table;
use crate::{
    Buffer, Caps, CommandPool, IndirectBuffer, QueryPool, Texture, TextureInfo, cull_mode, format,
    front_face, index_type, load_op, memory, query_kind, slot, stage, store_op, texture_dimension,
    texture_layout, texture_usage, topology,
};

pub struct Check<'a> {
    caps: &'a Caps,
    table: &'a Table,
    pool: CommandPool,
    base: &'static str,
    open: &'static str,
    index: Option<(Buffer, u64, u32)>,
    mesh: bool,
    indirect: bool,
    compute: bool,
    render: bool,
    topology: u32,
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
    carried: u32,
    fresh: u32,
    barrier: bool,
    passes: bool,
    resident: bool,
}

const STATES: [&str; 6] = [
    "set_cull",
    "set_face",
    "set_depth_state",
    "set_stencil_reference",
    "set_depth_bias",
    "set_blend_color",
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

pub(crate) fn assert_known(name: &str, field: &str, text: String, valid: bool) {
    check!(valid, "{name} {field} is {text}");
}

fn assert_bits(name: &str, field: &str, value: u32, all: u32, group: &str) {
    check_eq!(
        value & !all,
        0,
        "{name} {field} has bits {:#x} that are no {group} constant",
        value & !all
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

fn assert_aligned(name: &str, offset: u64, alignment: u32) {
    check!(
        offset.is_multiple_of(alignment.max(1) as u64),
        "{name} offset {offset} is not a multiple of caps.bind_alignment {alignment}"
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

fn copy_block_size(f: u32) -> u32 {
    match f {
        format::D24_UNORM_S8 | format::D32_FLOAT_S8 => 4,
        f => format::block(f)[0],
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
        check!(
            !busy.get(),
            "begin_commands on {pool}, which is already recording a command buffer; submit, finish or drop that one first"
        );
        busy.set(true);
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
            topology: topology::TRIANGLE,
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
            carried: 0,
            fresh: 0,
            barrier: false,
            passes: false,
            resident: base != "outside",
        }
    }

    pub fn queue(&self) -> usize {
        self.table.pools.get(self.pool).0
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
        check!(
            !self.barrier,
            "a barrier with no pass before or after it in its command buffer orders nothing on Metal; record it next to the pass that writes or reads the results"
        );
    }

    fn require(&self, c: &Command, has: bool, cap: &str) -> Result<(), String> {
        if has {
            Ok(())
        } else {
            Err(format!("{} needs caps.{cap}", command_name(c)))
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
        self.assert_resident(name, &buffer, self.table.residency.buffer(buffer));
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
    ) -> Result<(), String> {
        let size = self.buffer_range(&command_name(c), buffer, offset, size);
        check!(size > 0, "{} covers 0 bytes of {buffer}", command_name(c));
        if size > max {
            return Err(format!(
                "{} binds {size} bytes, over the limit of {max}",
                command_name(c)
            ));
        }
        Ok(())
    }

    fn check_barrier(&mut self, name: &str, src: u32, dst: u32, layouts: (u32, u32)) {
        assert_bits(name, "src_stage", src, stage::BITS, "stage");
        assert_bits(name, "dst_stage", dst, stage::BITS, "stage");
        check!(dst != 0, "{name} needs a dst_stage");
        check!(
            src != 0 || layouts.0 == texture_layout::UNDEFINED,
            "{name} needs a src_stage; only a texture_barrier from texture_layout::UNDEFINED waits on nothing"
        );
        let moves = layouts.0 != layouts.1;
        if self.open == "outside" && !self.passes && !moves && dst & !stage::HOST != 0 {
            self.barrier = true;
        }
    }

    fn texture_with_usage(
        &self,
        name: &str,
        texture: Texture,
        needs: u32,
        what: &str,
    ) -> &TextureInfo {
        let t = self.table.textures.get(texture);
        let copies = if t.usage & texture_usage::HOST != 0 {
            texture_usage::COPY_SRC | texture_usage::COPY_DST
        } else {
            0
        };
        check_ne!(
            (t.usage | copies) & needs,
            0,
            "{name} needs {texture} created with texture_usage::{what}"
        );
        self.table.assert_defined(name, texture);
        let root = self.table.parents.find(texture).copied().unwrap_or(texture);
        self.assert_resident(name, &texture, self.table.residency.texture(root));
        t
    }

    fn texture_region(
        &self,
        name: &str,
        texture: Texture,
        (mip, layer, layers): (u32, u32, u32),
        at: [u32; 3],
        size: [u32; 3],
    ) -> &TextureInfo {
        let t = self.table.textures.get(texture);
        assert_span(name, "mip", mip, 1, t.mips, texture);
        assert_span(name, "layer", layer, layers, t.layers, texture);
        let end = [t.width, t.height, t.depth].map(|n| (n >> mip).max(1));
        check!(
            (0..3).all(|i| at[i] as u64 + size[i] as u64 <= end[i] as u64),
            "{name} region {at:?} {size:?} is outside mip {mip} of {texture} ({end:?})"
        );
        let [_, bw, bh] = format::block(t.format);
        let edge = |i: usize, b: u32| size[i].is_multiple_of(b) || at[i] + size[i] == end[i];
        check!(
            at[0].is_multiple_of(bw) && at[1].is_multiple_of(bh) && edge(0, bw) && edge(1, bh),
            "{name} region {at:?} {size:?} is not aligned to the {bw}x{bh} blocks of {texture}"
        );
        t
    }

    fn query_range(&self, name: &str, pool: QueryPool, kinds: &[u32], first: u32, count: u32) {
        let &(kind, total) = self.table.queries.get(pool);
        self.assert_resident(name, &pool, self.table.residency.query(pool));
        let names: Vec<_> = kinds.iter().map(|&k| query_kind::name(k)).collect();
        check!(
            kinds.is_empty() || kinds.contains(&kind),
            "{name} needs a {} pool, but {pool} is {}",
            names.join(" or "),
            query_kind::name(kind)
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

    fn check_draw(&self, c: &Command, indexed: bool) {
        check!(
            self.render,
            "{} needs bind_render_pipeline earlier in this pass",
            command_name(c)
        );
        check!(
            self.views > 0 && self.scissors > 0,
            "{} needs set_viewport and set_scissor earlier in this pass; passes start with neither",
            command_name(c)
        );
        check_eq!(
            self.views,
            self.scissors,
            "{} has {} viewports but {} scissors; set one scissor per viewport",
            command_name(c),
            self.views,
            self.scissors
        );
        let stale = self.carried & !self.fresh;
        if stale != 0 {
            let state = STATES[stale.trailing_zeros() as usize];
            panic!(
                "{} needs {state} again in this pass; an earlier pass set it, and state does not carry between passes",
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
                "{} reads attachment {i}; give the pass a texture_usage::INPUT color there",
                command_name(c)
            );
        }
    }

    fn indirect_list(&self, c: &Command, indirect: IndirectBuffer) -> Result<u32, String> {
        let name = &command_name(c);
        let e = *self.table.indirects.get(indirect);
        self.assert_resident(name, &indirect, self.table.residency.indirect(indirect));
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
            "{name} draws with topology {} but {indirect} holds topology {}",
            topology::name(self.topology),
            topology::name(e.topology)
        );
        if e.info.indexed {
            let (buffer, offset, kind) = (e.info.indices, e.info.offset, e.info.kind);
            check_eq!(
                self.index,
                Some((buffer, offset, kind)),
                "{name} needs bind_index_buffer of {buffer} at offset {offset} with {}",
                index_type::name(kind)
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

    fn check_transient(&self, name: &str, texture: Texture, load: u32, store: u32) {
        if self.table.textures.get(texture).memory == memory::TRANSIENT {
            check!(
                load != load_op::LOAD && store == store_op::DONT_CARE,
                "{name} loads or stores memory::TRANSIENT {texture}; use load_op::CLEAR or DONT_CARE and store_op::DONT_CARE"
            );
        }
    }

    fn check_indirect_buffer(&self, name: &str, buffer: Buffer, offset: u64, size: u64) {
        check!(
            offset.is_multiple_of(4),
            "{name} offset {offset} is not a multiple of 4"
        );
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
            (texture_usage::COPY_DST, "COPY_DST")
        } else {
            (texture_usage::COPY_SRC, "COPY_SRC")
        };
        let name = command_name(c);
        let t = self.texture_with_usage(&name, texture, needs, what);
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
        let [_, bw, bh] = format::block(t.format);
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

    pub fn step(&mut self, c: &Command) -> Result<(), String> {
        if !cfg!(feature = "validation") {
            return Ok(());
        }
        let scope = c.scope();
        let open = self.open;
        let name = &command_name(c);
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
            } => {
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
                        "begin_render_pass {width}x{height} clears {texture} ({}x{}) but does not cover it; match its size or use load_op::LOAD",
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
                self.barrier = false;
                self.passes = true;
                self.render = false;
                self.views = 0;
                self.scissors = 0;
                self.carried |= self.fresh;
                self.fresh = 0;
            }
            Command::BeginComputePass => {
                self.open = "compute";
                self.barrier = false;
                self.passes = true;
                self.compute = false;
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
                self.open = "outside";
                self.targets.clear();
                self.resolves.clear();
                self.heap = None;
                self.inputs = [Texture::default(); slot::COLORS as usize];
                self.tile = None;
            }
            Command::BindComputePipeline { pipeline } => {
                self.require(c, caps.compute, "compute")?;
                self.table.pipelines.get(pipeline);
                self.compute = true;
            }
            Command::Dispatch { .. } => {
                self.require(c, caps.compute, "compute")?;
                check!(
                    self.compute,
                    "dispatch needs bind_compute_pipeline earlier in this pass"
                );
            }
            Command::DispatchIndirect { buffer, offset } => {
                self.require(c, caps.compute, "compute")?;
                check!(
                    self.compute,
                    "dispatch_indirect needs bind_compute_pipeline earlier in this pass"
                );
                check!(
                    offset.is_multiple_of(4),
                    "dispatch_indirect offset {offset} is not a multiple of 4"
                );
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
                self.buffer_range(name, src, src_offset, size);
                self.buffer_range(name, dst, dst_offset, size);
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
                self.limited_range(c, buffer, offset, size, !0)?;
            }
            Command::MemoryBarrier {
                src_stage,
                dst_stage,
            } => {
                let general = (texture_layout::GENERAL, texture_layout::GENERAL);
                self.check_barrier(name, src_stage, dst_stage, general)
            }
            Command::TextureBarrier {
                texture,
                src_stage,
                dst_stage,
                before,
                after,
                base_mip,
                mips,
                base_layer,
                layers,
            } => {
                self.check_barrier(name, src_stage, dst_stage, (before, after));
                let layout = |v| (texture_layout::name(v), texture_layout::valid(v));
                let (text, valid) = layout(before);
                assert_known(name, "before", text, valid);
                let (text, valid) = layout(after);
                assert_known(name, "after", text, valid);
                check_ne!(
                    after,
                    texture_layout::UNDEFINED,
                    "{name} after is texture_layout::UNDEFINED; transition to a defined layout"
                );
                if before == texture_layout::UNDEFINED {
                    self.table.define(texture);
                } else {
                    self.table.assert_defined(name, texture);
                }
                let t = self.table.textures.get(texture);
                assert_span(name, "mip", base_mip, mips, t.mips, texture);
                assert_span(name, "layer", base_layer, layers, t.layers, texture);
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
                self.texture_with_usage(name, src, texture_usage::COPY_SRC, "COPY_SRC");
                self.texture_with_usage(name, dst, texture_usage::COPY_DST, "COPY_DST");
                let size = [width, height, d];
                check!(
                    size.iter().all(|&n| n > 0),
                    "copy_texture region {width}x{height}x{d} has a zero"
                );
                let from = (src_mip, src_layer, layers);
                let s = self.texture_region(name, src, from, [src_x, src_y, src_z], size);
                let to = (dst_mip, dst_layer, layers);
                let t = self.texture_region(name, dst, to, [dst_x, dst_y, dst_z], size);
                check_eq!(
                    format::block(s.format),
                    format::block(t.format),
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
                        texture_dimension::D1 | texture_dimension::D1_ARRAY
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
                assert_known(name, "load", load_op::name(ld), load_op::valid(ld));
                assert_known(name, "store", store_op::name(st), store_op::valid(st));
                let t = self.texture_with_usage(name, texture, texture_usage::COLOR, "COLOR");
                if t.usage & texture_usage::INPUT != 0 {
                    self.inputs[index as usize] = texture;
                }
                self.check_transient(name, texture, ld, st);
                self.targets.push((texture, ld == load_op::CLEAR));
                if !resolve.is_null() {
                    self.resolves.push(resolve);
                    let r = self.texture_with_usage(name, resolve, texture_usage::COLOR, "COLOR");
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
                        !format::uint(s.format) && !format::sint(s.format),
                        "{name} resolves integer format {}, which no backend can average",
                        s.format
                    );
                }
            }
            Command::DepthAttachment {
                texture,
                load: ld,
                store: st,
                depth,
                stencil_load,
                stencil_store,
                ..
            } => {
                assert_known(name, "load", load_op::name(ld), load_op::valid(ld));
                assert_known(name, "store", store_op::name(st), store_op::valid(st));
                let (text, valid) = (load_op::name(stencil_load), load_op::valid(stencil_load));
                assert_known(name, "stencil_load", text, valid);
                let (text, valid) = (
                    store_op::name(stencil_store),
                    store_op::valid(stencil_store),
                );
                assert_known(name, "stencil_store", text, valid);
                check!(
                    (0.0..=1.0).contains(&depth),
                    "{name} clear depth {depth} is outside 0..1"
                );
                self.texture_with_usage(name, texture, texture_usage::DEPTH, "DEPTH");
                self.check_transient(name, texture, ld, st);
                self.check_transient(name, texture, stencil_load, stencil_store);
                let clear = ld == load_op::CLEAR || stencil_load == load_op::CLEAR;
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
                    return Err(format!(
                        "{name} shared memory {shared} is over caps.max_shared {}",
                        caps.max_shared
                    ));
                }
                self.tile = Some((width, height));
            }
            Command::DispatchTile { pipeline } => {
                self.require(c, caps.tiles, "tiles")?;
                let samples = *self.table.tile_pipelines.get(pipeline);
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
            Command::SetCullMode { cull: v } => {
                assert_known(name, "cull", cull_mode::name(v), cull_mode::valid(v));
                self.mark_state(0);
            }
            Command::SetFrontFace { face: v } => {
                assert_known(name, "face", front_face::name(v), front_face::valid(v));
                self.mark_state(1);
            }
            Command::SetStencilReference { .. } => self.mark_state(3),
            Command::SetBlendColor { .. } => self.mark_state(5),
            Command::SetPrimitiveTopology { topology: v } => {
                assert_known(name, "topology", topology::name(v), topology::valid(v));
                self.topology = v;
            }
            Command::SetScissor {
                x,
                y,
                width,
                height,
            } => {
                let end = (x as u64 + width as u64).max(y as u64 + height as u64);
                check!(
                    end <= i32::MAX as u64,
                    "set_scissor {x},{y} {width}x{height} ends past i32::MAX"
                );
                self.scissors = 1;
            }
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
                self.table.residency.assert_set(set);
                self.resident = true;
            }
            Command::BindUniformBuffer {
                slot: i,
                buffer,
                offset,
                size,
            } => {
                self.require(c, caps.slots, "slots")?;
                assert_slot(name, i, slot::UNIFORMS);
                assert_aligned(name, offset, caps.bind_alignment);
                self.limited_range(c, buffer, offset, size, caps.max_uniform)?;
            }
            Command::BindStorageBuffer {
                slot: i,
                buffer,
                offset,
                size,
            } => {
                self.require(c, caps.slots, "slots")?;
                self.require(c, caps.storage, "storage")?;
                assert_slot(name, i, slot::STORAGES);
                assert_aligned(name, offset, caps.bind_alignment);
                self.limited_range(c, buffer, offset, size, caps.max_storage)?;
            }
            Command::BindTexture { slot: i, texture } => {
                self.require(c, caps.slots, "slots")?;
                assert_slot(name, i, slot::TEXTURES);
                self.texture_with_usage(name, texture, texture_usage::SAMPLED, "SAMPLED");
            }
            Command::BindStorageTexture { slot: i, texture } => {
                self.require(c, caps.slots, "slots")?;
                self.require(c, caps.storage, "storage")?;
                assert_slot(name, i, slot::IMAGES);
                self.texture_with_usage(name, texture, texture_usage::STORAGE, "STORAGE");
            }
            Command::BindSampler { slot: i, sampler } => {
                self.require(c, caps.slots, "slots")?;
                assert_slot(name, i, slot::SAMPLERS);
                self.table.samplers.get(sampler);
            }
            Command::BindVertexBuffer {
                slot: i,
                buffer,
                offset,
            } => {
                assert_slot(name, i, slot::VERTICES);
                self.buffer_range(name, buffer, offset, 0);
                self.vertices |= 1 << i;
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => {
                assert_known(
                    name,
                    "kind",
                    index_type::name(kind),
                    index_type::valid(kind),
                );
                let size = if kind == index_type::U16 { 2 } else { 4 };
                check!(
                    offset.is_multiple_of(size),
                    "bind_index_buffer offset {offset} is not a multiple of {size}"
                );
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
                self.buffer_range(name, buffer, offset, 8);
                self.require(c, caps.indirect_count, "indirect_count")?;
            }
            Command::DrawMeshTasks { x, y, z } => {
                self.require(c, caps.mesh, "mesh")?;
                self.check_draw(c, false);
                let total = x as u64 * y as u64 * z as u64;
                if x.max(y).max(z) > caps.mesh_axis || total > caps.mesh_groups as u64 {
                    return Err(format!(
                        "{name} grid {x}x{y}x{z} is over caps.mesh_axis {} or caps.mesh_groups {}",
                        caps.mesh_axis, caps.mesh_groups
                    ));
                }
            }
            Command::DrawMeshTasksIndirect { buffer, offset } => {
                self.require(c, caps.mesh_indirect, "mesh_indirect")?;
                self.check_draw(c, false);
                self.check_indirect_buffer(name, buffer, offset, 12);
            }
            Command::OcclusionPool { pool } => {
                let kinds = [query_kind::OCCLUSION, query_kind::PRECISE];
                self.query_range(name, pool, &kinds, 0, 0);
                self.heap = Some(pool);
            }
            Command::ResetQueries { pool, first, count } => {
                self.query_range(name, pool, &[], first, count);
            }
            Command::WriteTimestamp { pool, index, stage } => {
                check_eq!(
                    stage.count_ones(),
                    1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
                assert_bits(name, "stage", stage, stage::BITS, "stage");
                self.require(c, caps.timestamps, "timestamps")?;
                self.query_range(name, pool, &[query_kind::TIMESTAMP], index, 1);
            }
            Command::BeginOcclusionQuery { pool, index }
            | Command::EndOcclusionQuery { pool, index } => {
                self.require(c, caps.occlusion, "occlusion")?;
                check_eq!(
                    self.heap,
                    Some(pool),
                    "{name} needs RenderPassInfo.occlusion set to {pool} in begin_render_pass"
                );
                let kinds = [query_kind::OCCLUSION, query_kind::PRECISE];
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
                let kinds: &[u32] = if matches!(c, Command::ResolveTimestamps { .. }) {
                    &[query_kind::TIMESTAMP]
                } else {
                    &[query_kind::OCCLUSION, query_kind::PRECISE]
                };
                self.query_range(name, pool, kinds, first, count);
                check!(
                    offset.is_multiple_of(8),
                    "{name} offset {offset} is not a multiple of 8"
                );
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
            Command::SetViewports { viewports } => {
                assert_rects(name, viewports.len(), caps.viewports);
                for v in viewports {
                    assert_viewport(name, v.x, v.y, v.width, v.height, v.near, v.far);
                }
                self.views = viewports.len() as u32;
            }
            Command::SetViewport {
                x,
                y,
                width,
                height,
                near,
                far,
            } => {
                assert_viewport(name, x, y, width, height, near, far);
                self.views = 1;
            }
            Command::SetScissors { scissors } => {
                assert_rects(name, scissors.len(), caps.viewports);
                for r in scissors {
                    let end = (r.x as u64 + r.width as u64).max(r.y as u64 + r.height as u64);
                    check!(
                        end <= i32::MAX as u64,
                        "set_scissors {r:?} ends past i32::MAX"
                    );
                }
                self.scissors = scissors.len() as u32;
            }
        }
        Ok(())
    }
}

impl Drop for Check<'_> {
    fn drop(&mut self) {
        if let Some(pool) = self.table.pools.find(self.pool) {
            pool.1.set(false);
        }
    }
}
