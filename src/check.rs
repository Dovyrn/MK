use crate::caps::{compressed, depth};
use crate::table::Table;
use crate::{
    Caps, Command, TextureInfo, access, cull, dimension, face, filter, format, index, layout, load,
    query, slot, stage, store, topology, usage,
};

pub struct Check<'a> {
    caps: &'a Caps,
    table: &'a Table,
    natives: usize,
    open: &'static str,
    index: Option<(u32, u64, u32)>,
    mesh: bool,
    compute: bool,
    render: bool,
    indirect: bool,
    topology: u32,
    heap: Option<u32>,
    targets: Vec<(u32, bool)>,
    samples: u32,
    bound: u32,
    labels: Vec<&'static str>,
    needs: u32,
    vertices: u32,
    resolves: Vec<u32>,
}

fn viewport(x: f32, y: f32, width: f32, height: f32, near: f32, far: f32) {
    assert!(
        [x, y, width, height].iter().all(|v| v.is_finite()) && width > 0.0 && height > 0.0,
        "viewport {x},{y} {width}x{height} is not a finite positive area"
    );
    assert!(
        (0.0..=1.0).contains(&near) && (0.0..=1.0).contains(&far),
        "viewport depth range {near}..{far} is outside 0..1"
    );
}

fn known(what: &str, value: u32, valid: bool) {
    assert!(valid, "unknown {what} {value}");
}

fn bits(what: &str, value: u32, all: u32) {
    assert_eq!(value & !all, 0, "unknown {what} bits {:#x}", value & !all);
}

fn name(c: &Command) -> String {
    c.to_string().split(' ').next().unwrap().to_string()
}

fn rects(data: &[u8], size: usize, count: u32) {
    assert!(
        data.len().is_multiple_of(size),
        "rect data is not a multiple of {size} bytes"
    );
    assert!(
        data.len() / size <= count as usize,
        "{} rects is more than {count} viewports",
        data.len() / size
    );
}

fn aligned(offset: u64, alignment: u32) {
    assert!(
        offset.is_multiple_of(alignment.max(1) as u64),
        "bind offset {offset} is not a multiple of caps.bind_alignment {alignment}"
    );
}

fn slot(slot: u32, count: u32) {
    assert!(slot < count, "slot {slot} is outside 0..{count}");
}

fn span(what: &str, base: u32, count: u32, total: u32, texture: u32) {
    assert_ne!(count, 0, "{what} count is 0 for texture {texture}");
    let end = if count == !0 {
        total
    } else {
        base.saturating_add(count)
    };
    assert!(
        base < total && end <= total,
        "{what}s {base}..{end} are outside 0..{total} of texture {texture}"
    );
}

fn needs_layout(what: &str, l: u32, want: u32) {
    assert!(
        l == want || l == layout::GENERAL,
        "{what} layout {l} must be {want} or GENERAL"
    );
}

fn apart(a: u32, b: u32, n: u32) -> bool {
    a as u64 + n as u64 <= b as u64 || b as u64 + n as u64 <= a as u64
}

fn copied(f: u32) -> u32 {
    match f {
        format::D24_UNORM_S8 | format::D32_FLOAT_S8 => 4,
        f => format::block(f)[0],
    }
}

impl<'a> Check<'a> {
    pub fn run(
        mut bytes: &[u8],
        natives: usize,
        caps: &'a Caps,
        table: &'a Table,
    ) -> Result<(), String> {
        let mut check = Check {
            caps,
            table,
            natives,
            open: "outside",
            index: None,
            mesh: false,
            compute: false,
            render: false,
            indirect: false,
            topology: topology::TRIANGLE,
            heap: None,
            targets: Vec::new(),
            samples: 0,
            bound: 0,
            labels: Vec::new(),
            needs: 0,
            vertices: 0,
            resolves: Vec::new(),
        };
        while !bytes.is_empty() {
            let c = Command::read(&mut bytes)
                .unwrap_or_else(|name| panic!("{name} is not in this backend"));
            check.step(&c)?;
        }
        assert_eq!(
            check.open, "outside",
            "stream ends inside begin_{}",
            check.open
        );
        Ok(())
    }

    fn need(&self, c: &Command, has: bool, cap: &str) -> Result<(), String> {
        if has {
            Ok(())
        } else {
            Err(format!("{} needs caps.{cap}", name(c)))
        }
    }

    fn range(&self, buffer: u32, offset: u64, size: u64) -> u64 {
        let total = *self.table.buffers.get(buffer);
        let end = if size == !0 {
            total
        } else {
            offset.saturating_add(size)
        };
        assert!(
            offset <= total && end <= total,
            "range {offset}..{end} is past buffer {buffer} ({total} bytes)"
        );
        end - offset
    }

    fn filled(
        &self,
        c: &Command,
        buffer: u32,
        offset: u64,
        size: u64,
        max: u64,
    ) -> Result<(), String> {
        let size = self.range(buffer, offset, size);
        assert!(size > 0, "{} covers 0 bytes", name(c));
        if size > max {
            return Err(format!(
                "{} binds {size} bytes, over the limit of {max}",
                name(c)
            ));
        }
        Ok(())
    }

    fn barrier(&self, src_stage: u32, src_access: u32, dst_stage: u32, dst_access: u32) {
        bits("stage", src_stage | dst_stage, stage::BITS);
        bits("access", src_access | dst_access, access::BITS);
    }

    fn texture(&self, texture: u32, needs: u32, what: &str) -> &TextureInfo {
        let t = self.table.textures.get(texture);
        let copies = if t.usage & usage::HOST != 0 {
            usage::COPY_SRC | usage::COPY_DST
        } else {
            0
        };
        assert_ne!(
            (t.usage | copies) & needs,
            0,
            "texture {texture} has no usage::{what}"
        );
        t
    }

    fn region(
        &self,
        texture: u32,
        (mip, layer, layers): (u32, u32, u32),
        at: [u32; 3],
        size: [u32; 3],
    ) -> &TextureInfo {
        let t = self.table.textures.get(texture);
        span("mip", mip, 1, t.mips, texture);
        span("layer", layer, layers, t.layers, texture);
        let end = [t.width, t.height, t.depth].map(|n| (n >> mip).max(1));
        assert!(
            (0..3).all(|i| at[i] as u64 + size[i] as u64 <= end[i] as u64),
            "region {at:?} {size:?} is outside mip {mip} of texture {texture} ({end:?})"
        );
        let [_, bw, bh] = format::block(t.format);
        let edge = |i: usize, b: u32| size[i].is_multiple_of(b) || at[i] + size[i] == end[i];
        assert!(
            at[0].is_multiple_of(bw) && at[1].is_multiple_of(bh) && edge(0, bw) && edge(1, bh),
            "region {at:?} {size:?} is not aligned to the {bw}x{bh} blocks of texture {texture}"
        );
        t
    }

    fn queries(&self, heap: u32, kinds: &[u32], first: u32, count: u32) {
        let &(kind, total) = self.table.queries.get(heap);
        assert!(
            kinds.is_empty() || kinds.contains(&kind),
            "query heap {heap} has kind {kind}, not {kinds:?}"
        );
        if count > 0 {
            let last = first.saturating_add(count - 1);
            assert!(last < total, "query {last} is outside 0..{total}");
        }
    }

    fn list(&self, c: &Command, indirect: u32) -> Result<u32, String> {
        let (info, topology, mesh) = *self.table.indirects.get(indirect);
        self.need(c, self.caps.indirect, "indirect")?;
        if mesh {
            self.need(c, self.caps.mesh_indirect, "mesh_indirect")?;
        }
        self.draw(c, info.indexed);
        assert!(
            self.indirect,
            "{c} needs a render pipeline created with indirect: true"
        );
        assert_eq!(
            self.mesh, mesh,
            "{c} of indirect buffer {indirect} needs a mesh pipeline exactly when it was made with one"
        );
        assert_eq!(
            self.topology, topology,
            "{c} draws with topology {} but indirect buffer {indirect} holds topology {topology}",
            self.topology
        );
        if info.indexed {
            let (buffer, offset, kind) = (info.indices, info.offset, info.kind);
            assert_eq!(
                self.index,
                Some((buffer, offset, kind)),
                "{c} needs index buffer {buffer} bound at offset {offset} with kind {kind}"
            );
            if offset != 0 {
                self.need(c, self.caps.indirect_offset, "indirect_offset")?;
            }
        }
        if mesh {
            self.need(c, self.caps.mesh, "mesh")?;
        }
        Ok(info.size)
    }

    fn draw(&self, c: &Command, indexed: bool) {
        assert!(self.render, "{} needs bind_render_pipeline first", name(c));
        assert!(
            self.samples == 0 || self.bound == self.samples,
            "{} draws with a {}-sample pipeline in a {}-sample pass",
            name(c),
            self.bound,
            self.samples
        );
        let missing = self.needs & !self.vertices;
        assert_eq!(
            missing,
            0,
            "{} needs vertex buffers {missing:#b} bound for the pipeline's attributes",
            name(c)
        );
        assert!(
            !indexed || self.index.is_some(),
            "{} needs bind_index_buffer first",
            name(c)
        );
    }

    fn indirect(&self, buffer: u32, offset: u64, count: u32, stride: u32, size: u64) {
        assert!(
            offset.is_multiple_of(4),
            "indirect offset {offset} is not a multiple of 4"
        );
        assert!(
            stride == 0 || (stride.is_multiple_of(4) && stride as u64 >= size),
            "indirect stride {stride} must be 0 or a multiple of 4 of at least {size}"
        );
        let stride = if stride == 0 { size } else { stride as u64 };
        let span = stride * count.saturating_sub(1) as u64 + size;
        self.range(buffer, offset, if count == 0 { 0 } else { span });
    }

    fn copy(&self, c: &Command) {
        let (Command::CopyBufferToTexture {
            buffer,
            offset,
            row_length,
            row_count,
            texture,
            layout,
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
            layout,
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
        let (needs, what, expect) = if upload {
            (usage::COPY_DST, "COPY_DST", layout::COPY_DST)
        } else {
            (usage::COPY_SRC, "COPY_SRC", layout::COPY_SRC)
        };
        let t = self.texture(texture, needs, what);
        assert!(
            layout == expect || layout == layout::GENERAL,
            "{} layout {layout} must be {what} or GENERAL",
            name(c)
        );
        assert!(
            w > 0 && h > 0 && d > 0,
            "{} region {w}x{h}x{d} has a zero",
            name(c)
        );
        span("mip", mip, 1, t.mips, texture);
        span("layer", layer, 1, t.layers, texture);
        let size = [
            (t.width >> mip).max(1),
            (t.height >> mip).max(1),
            (t.depth >> mip).max(1),
        ];
        let fits = |at: u32, n: u32, end: u32| at as u64 + n as u64 <= end as u64;
        assert!(
            fits(x, w, size[0]) && fits(y, h, size[1]) && fits(z, d, size[2]),
            "region {x},{y},{z} {w}x{h}x{d} is outside mip {mip} of texture {texture} ({}x{}x{})",
            size[0],
            size[1],
            size[2]
        );
        let [_, bw, bh] = format::block(t.format);
        let edge = |at: u32, n: u32, b: u32, end: u32| {
            at.is_multiple_of(b) && (n.is_multiple_of(b) || at + n == end)
        };
        assert!(
            edge(x, w, bw, size[0]) && edge(y, h, bh, size[1]),
            "{} region {x},{y} {w}x{h} is not aligned to the {bw}x{bh} blocks of texture {texture}",
            name(c)
        );
        assert!(
            row_length.is_multiple_of(bw) && row_count.is_multiple_of(bh),
            "{} rows {row_length}x{row_count} are not multiples of the {bw}x{bh} blocks",
            name(c)
        );
        let texel = copied(t.format).max(4) as u64;
        assert!(
            offset.is_multiple_of(texel),
            "{} buffer offset {offset} is not a multiple of {texel}",
            name(c)
        );
        let row = if row_length == 0 { w } else { row_length };
        let rows = if row_count == 0 { h } else { row_count };
        assert!(
            row >= w && rows >= h,
            "{} rows {row}x{rows} are smaller than the region {w}x{h}",
            name(c)
        );
        let bytes = row.div_ceil(bw) as u64 * copied(t.format) as u64;
        let total = bytes
            .checked_mul(rows.div_ceil(bh) as u64)
            .and_then(|n| n.checked_mul(d as u64))
            .unwrap_or_else(|| panic!("{} region size overflows", name(c)));
        self.range(buffer, offset, total);
    }

    fn step(&mut self, c: &Command) -> Result<(), String> {
        let scope = c.scope();
        let open = self.open;
        assert!(
            scope.is_empty() || scope.split(' ').any(|s| s == open),
            "{c} runs only in {scope}, not {open}"
        );
        let caps = self.caps;
        match *c {
            Command::BeginRender {
                width,
                height,
                layers,
            } => {
                assert!(
                    width > 0 && height > 0 && layers > 0,
                    "begin_render size {width}x{height}x{layers} has a zero"
                );
                assert_eq!(open, "outside", "{c} inside begin_{open}");
                let samples: Vec<u32> = self
                    .targets
                    .iter()
                    .map(|&(t, _)| self.table.textures.get(t).samples)
                    .collect();
                assert!(
                    samples.windows(2).all(|w| w[0] == w[1]),
                    "begin_render attachments have different sample counts {samples:?}"
                );
                self.samples = samples.first().copied().unwrap_or(0);
                for &(texture, clear) in &self.targets {
                    let t = self.table.textures.get(texture);
                    assert!(
                        width <= t.width && height <= t.height && layers <= t.layers,
                        "begin_render {width}x{height}x{layers} is larger than texture {texture} ({}x{}x{})",
                        t.width,
                        t.height,
                        t.layers
                    );
                    assert!(
                        !clear || (width == t.width && height == t.height),
                        "begin_render {width}x{height} must cover texture {texture} ({}x{}) to clear it",
                        t.width,
                        t.height
                    );
                }
                for &r in &self.resolves {
                    let t = self.table.textures.get(r);
                    if width < t.width || height < t.height {
                        self.need(c, caps.partial_resolve, "partial_resolve")?;
                    }
                }
                self.open = "render";
            }
            Command::BeginCompute => {
                assert_eq!(open, "outside", "{c} inside begin_{open}");
                self.open = "compute";
            }
            Command::EndCompute => {
                assert_eq!(open, "compute", "end_compute without begin_compute");
                assert_ne!(
                    self.labels.last(),
                    Some(&"compute"),
                    "end_compute with a label still open inside it"
                );
                self.open = "outside";
            }
            Command::EndRender => {
                assert_eq!(open, "render", "end_render without begin_render");
                assert_ne!(
                    self.labels.last(),
                    Some(&"render"),
                    "end_render with a label still open inside it"
                );
                self.open = "outside";
                self.targets.clear();
                self.resolves.clear();
                self.heap = None;
            }
            Command::BindPipeline { pipeline } => {
                self.need(c, caps.compute, "compute")?;
                self.table.pipelines.get(pipeline);
                self.compute = true;
            }
            Command::Dispatch { .. } => {
                self.need(c, caps.compute, "compute")?;
                assert!(self.compute, "dispatch needs bind_pipeline first");
            }
            Command::DispatchIndirect { buffer, offset } => {
                self.need(c, caps.compute, "compute")?;
                assert!(self.compute, "dispatch_indirect needs bind_pipeline first");
                assert!(
                    offset.is_multiple_of(4),
                    "dispatch_indirect offset {offset} is not a multiple of 4"
                );
                self.range(buffer, offset, 12);
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => {
                assert!(size > 0, "copy_buffer size is 0");
                self.range(src, src_offset, size);
                self.range(dst, dst_offset, size);
            }
            Command::FillBuffer {
                buffer,
                offset,
                size,
                ..
            } => {
                self.need(c, caps.buffer_clear, "buffer_clear")?;
                assert!(
                    offset.is_multiple_of(4) && (size == !0 || size.is_multiple_of(4)),
                    "fill_buffer offset {offset} and size {size} must be multiples of 4"
                );
                self.filled(c, buffer, offset, size, !0)?;
            }
            Command::Barrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            }
            | Command::QueueBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            } => self.barrier(src_stage, src_access, dst_stage, dst_access),
            Command::TextureBarrier {
                texture,
                src_stage,
                src_access,
                dst_stage,
                dst_access,
                before,
                after,
                base_mip,
                mips,
                base_layer,
                layers,
            } => {
                self.barrier(src_stage, src_access, dst_stage, dst_access);
                known("layout", before, layout::valid(before));
                known("layout", after, layout::valid(after));
                let t = self.table.textures.get(texture);
                span("mip", base_mip, mips, t.mips, texture);
                span("layer", base_layer, layers, t.layers, texture);
            }
            Command::CopyBufferToTexture { .. } | Command::CopyTextureToBuffer { .. } => {
                self.copy(c);
            }
            Command::CopyTexture {
                src,
                src_layout,
                src_mip,
                src_layer,
                src_x,
                src_y,
                src_z,
                dst,
                dst_layout,
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
                self.texture(src, usage::COPY_SRC, "COPY_SRC");
                self.texture(dst, usage::COPY_DST, "COPY_DST");
                needs_layout("copy_texture source", src_layout, layout::COPY_SRC);
                needs_layout("copy_texture destination", dst_layout, layout::COPY_DST);
                let size = [width, height, d];
                assert!(
                    size.iter().all(|&n| n > 0),
                    "copy_texture region {width}x{height}x{d} has a zero"
                );
                let from = (src_mip, src_layer, layers);
                let s = self.region(src, from, [src_x, src_y, src_z], size);
                let to = (dst_mip, dst_layer, layers);
                let t = self.region(dst, to, [dst_x, dst_y, dst_z], size);
                assert_eq!(
                    format::block(s.format),
                    format::block(t.format),
                    "copy_texture formats {} and {} have different blocks",
                    s.format,
                    t.format
                );
                assert!(
                    s.format == t.format || !depth(s.format) && !depth(t.format),
                    "copy_texture between depth formats {} and {} needs one format",
                    s.format,
                    t.format
                );
                assert_eq!(
                    s.samples, t.samples,
                    "copy_texture sample counts {} and {} differ",
                    s.samples, t.samples
                );
                assert!(
                    src != dst
                        || src_mip != dst_mip
                        || apart(src_layer, dst_layer, layers)
                        || apart(src_x, dst_x, width)
                        || apart(src_y, dst_y, height)
                        || apart(src_z, dst_z, d),
                    "copy_texture source and destination overlap in texture {src}"
                );
                let line =
                    |t: &TextureInfo| matches!(t.dimension, dimension::D1 | dimension::D1_ARRAY);
                assert_eq!(
                    line(s),
                    line(t),
                    "copy_texture cannot copy between 1D texture and 2D or 3D texture"
                );
                if s.format != t.format {
                    self.need(c, caps.copy_formats, "copy_formats")?;
                    assert_ne!(
                        s.usage & usage::VIEW,
                        0,
                        "copy_texture from format {} to {} needs usage::VIEW on texture {src}",
                        s.format,
                        t.format
                    );
                }
            }
            Command::BlitTexture {
                src,
                src_layout,
                src_mip,
                src_layer,
                src_x,
                src_y,
                src_width,
                src_height,
                dst,
                dst_layout,
                dst_mip,
                dst_layer,
                dst_x,
                dst_y,
                dst_width,
                dst_height,
                filter: f,
            } => {
                known("filter", f, filter::valid(f));
                self.texture(src, usage::BLIT, "BLIT");
                self.texture(dst, usage::BLIT, "BLIT");
                needs_layout("blit_texture source", src_layout, layout::COPY_SRC);
                needs_layout("blit_texture destination", dst_layout, layout::COPY_DST);
                assert!(
                    src_width.min(src_height).min(dst_width).min(dst_height) > 0,
                    "blit_texture region {src_width}x{src_height} to {dst_width}x{dst_height} has a zero"
                );
                let from = (src_mip, src_layer, 1);
                let s = self.region(src, from, [src_x, src_y, 0], [src_width, src_height, 1]);
                let to = (dst_mip, dst_layer, 1);
                let t = self.region(dst, to, [dst_x, dst_y, 0], [dst_width, dst_height, 1]);
                for (id, t) in [(src, s), (dst, t)] {
                    assert!(
                        matches!(
                            t.dimension,
                            dimension::D2
                                | dimension::D2_ARRAY
                                | dimension::CUBE
                                | dimension::CUBE_ARRAY
                        ) && t.samples == 1,
                        "blit_texture needs single-sampled 2D, 2D array or cube textures, not texture {id}"
                    );
                }
                let kind = |f| (format::uint(f), format::sint(f));
                assert_eq!(
                    kind(s.format),
                    kind(t.format),
                    "blit_texture formats {} and {} differ in integer kind",
                    s.format,
                    t.format
                );
                assert!(
                    f == filter::NEAREST || kind(s.format) == (false, false),
                    "blit_texture of integer format {} needs filter::NEAREST",
                    s.format
                );
                assert!(
                    src != dst || src_mip != dst_mip || src_layer != dst_layer,
                    "blit_texture reads and writes one subresource of texture {src}"
                );
            }
            Command::GenerateMips { texture, layout: l } => {
                let t = self.texture(texture, usage::MIPS, "MIPS");
                needs_layout("generate_mips", l, layout::COPY_DST);
                assert!(
                    !format::uint(t.format) && !format::sint(t.format),
                    "generate_mips needs a filterable format, not integer format {}",
                    t.format
                );
            }
            Command::ClearTexture {
                texture,
                layout: l,
                base_mip,
                mips,
                base_layer,
                layers,
                depth: value,
                ..
            } => {
                let t = self.texture(texture, usage::COPY_DST, "COPY_DST");
                needs_layout("clear_texture", l, layout::COPY_DST);
                span("mip", base_mip, mips, t.mips, texture);
                span("layer", base_layer, layers, t.layers, texture);
                assert!(
                    !compressed(t.format),
                    "clear_texture cannot clear compressed format {}",
                    t.format
                );
                assert!(
                    (0.0..=1.0).contains(&value),
                    "clear_texture depth {value} is outside 0..1"
                );
                if t.usage & (usage::COLOR | usage::DEPTH) == 0 {
                    self.need(c, caps.texture_clear, "texture_clear")?;
                }
            }
            Command::ColorAttachment {
                index,
                texture,
                layout: l,
                load: ld,
                store: st,
                resolve,
                resolve_layout,
                ..
            } => {
                slot(index, slot::COLORS);
                known("layout", l, layout::valid(l));
                known("load", ld, load::valid(ld));
                known("store", st, store::valid(st));
                self.texture(texture, usage::COLOR, "COLOR");
                self.targets.push((texture, ld == load::CLEAR));
                if resolve != !0 {
                    known("layout", resolve_layout, layout::valid(resolve_layout));
                    self.resolves.push(resolve);
                    let r = self.texture(resolve, usage::COLOR, "COLOR");
                    let s = self.table.textures.get(texture);
                    assert!(
                        s.samples > 1 && r.samples == 1,
                        "resolve needs multisampled texture {texture} and single-sample target {resolve}"
                    );
                    assert!(
                        s.format == r.format && s.width == r.width && s.height == r.height,
                        "resolve target {resolve} differs in format or size from texture {texture}"
                    );
                    assert!(
                        !format::uint(s.format) && !format::sint(s.format),
                        "integer format {} cannot be resolved",
                        s.format
                    );
                }
            }
            Command::DepthAttachment {
                texture,
                layout: l,
                load: ld,
                store: st,
                depth,
                stencil_load,
                stencil_store,
                ..
            } => {
                known("layout", l, layout::valid(l));
                known("load", ld, load::valid(ld));
                known("store", st, store::valid(st));
                known("load", stencil_load, load::valid(stencil_load));
                known("store", stencil_store, store::valid(stencil_store));
                assert!(
                    (0.0..=1.0).contains(&depth),
                    "depth clear value {depth} is outside 0..1"
                );
                self.texture(texture, usage::DEPTH, "DEPTH");
                let clear = ld == load::CLEAR || stencil_load == load::CLEAR;
                self.targets.push((texture, clear));
            }
            Command::BindRenderPipeline { pipeline } => {
                let (samples, indirect, shape, mesh, needs) = *self.table.renders.get(pipeline);
                assert!(
                    self.samples == 0 || samples == self.samples,
                    "render pipeline {pipeline} has {samples} samples but the pass has {}",
                    self.samples
                );
                self.render = true;
                self.bound = samples;
                self.needs = needs;
                self.indirect = indirect;
                self.topology = shape;
                self.mesh = mesh;
            }
            Command::SetCull { cull: v } => known("cull", v, cull::valid(v)),
            Command::SetFace { face: v } => known("face", v, face::valid(v)),
            Command::SetTopology { topology: v } => {
                known("topology", v, topology::valid(v));
                self.topology = v;
            }
            Command::SetScissor {
                x,
                y,
                width,
                height,
            } => {
                let end = (x as u64 + width as u64).max(y as u64 + height as u64);
                assert!(
                    end <= i32::MAX as u64,
                    "scissor {x},{y} {width}x{height} is too large"
                );
            }
            Command::Draw { .. } => self.draw(c, false),
            Command::DrawIndexed { .. } => self.draw(c, true),
            Command::SetDepthState { state } => {
                self.table.states.get(state);
            }
            Command::SetDepthBias { clamp, .. } => {
                if clamp != 0.0 {
                    self.need(c, caps.bias_clamp, "bias_clamp")?;
                }
            }
            Command::BindUniform {
                slot: i,
                buffer,
                offset,
                size,
            } => {
                self.need(c, caps.slots, "slots")?;
                slot(i, slot::UNIFORMS);
                aligned(offset, caps.bind_alignment);
                self.filled(c, buffer, offset, size, caps.max_uniform)?;
            }
            Command::BindStorage {
                slot: i,
                buffer,
                offset,
                size,
            } => {
                self.need(c, caps.slots, "slots")?;
                self.need(c, caps.storage, "storage")?;
                slot(i, slot::STORAGES);
                aligned(offset, caps.bind_alignment);
                self.filled(c, buffer, offset, size, caps.max_storage)?;
            }
            Command::BindTexture { slot: i, texture } => {
                self.need(c, caps.slots, "slots")?;
                slot(i, slot::TEXTURES);
                self.texture(texture, usage::SAMPLED, "SAMPLED");
            }
            Command::BindImage { slot: i, texture } => {
                self.need(c, caps.slots, "slots")?;
                self.need(c, caps.storage, "storage")?;
                slot(i, slot::IMAGES);
                self.texture(texture, usage::STORAGE, "STORAGE");
            }
            Command::BindSampler { slot: i, sampler } => {
                self.need(c, caps.slots, "slots")?;
                slot(i, slot::SAMPLERS);
                self.table.samplers.get(sampler);
            }
            Command::BindVertexBuffer {
                slot: i,
                buffer,
                offset,
            } => {
                slot(i, slot::VERTICES);
                self.range(buffer, offset, 0);
                self.vertices |= 1 << i;
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => {
                known("index kind", kind, index::valid(kind));
                let size = if kind == index::U16 { 2 } else { 4 };
                assert!(
                    offset.is_multiple_of(size),
                    "index buffer offset {offset} is not a multiple of {size}"
                );
                self.range(buffer, offset, 0);
                self.index = Some((buffer, offset, kind));
            }
            Command::DrawIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.need(c, caps.indirect, "indirect")?;
                if count > 1 {
                    self.need(c, caps.multi_draw, "multi_draw")?;
                }
                self.draw(c, false);
                self.indirect(buffer, offset, count, stride, 16);
            }
            Command::DrawIndexedIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.need(c, caps.indirect, "indirect")?;
                if count > 1 {
                    self.need(c, caps.multi_draw, "multi_draw")?;
                }
                self.draw(c, true);
                if self.index.map(|i| i.1) != Some(0) {
                    self.need(c, caps.indirect_offset, "indirect_offset")?;
                }
                self.indirect(buffer, offset, count, stride, 20);
            }
            Command::DrawMesh { x, y, z } => {
                self.need(c, caps.mesh, "mesh")?;
                self.draw(c, false);
                let total = x as u64 * y as u64 * z as u64;
                if x.max(y).max(z) > caps.mesh_axis || total > caps.mesh_groups as u64 {
                    return Err(format!(
                        "{c} grid {x}x{y}x{z} is over caps.mesh_axis {} or caps.mesh_groups {}",
                        caps.mesh_axis, caps.mesh_groups
                    ));
                }
            }
            Command::DrawMeshIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.need(c, caps.mesh_indirect, "mesh_indirect")?;
                if count > 1 {
                    self.need(c, caps.multi_draw, "multi_draw")?;
                }
                self.draw(c, false);
                self.indirect(buffer, offset, count, stride, 12);
            }
            Command::OcclusionHeap { heap } => {
                self.queries(heap, &[query::OCCLUSION, query::PRECISE], 0, 0);
                self.heap = Some(heap);
            }
            Command::ResetQueries { heap, first, count } => {
                self.queries(heap, &[], first, count);
            }
            Command::WriteTimestamp { heap, index, stage } => {
                assert_eq!(
                    stage.count_ones(),
                    1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
                bits("stage", stage, stage::BITS);
                self.need(c, caps.timestamps, "timestamps")?;
                self.queries(heap, &[query::TIMESTAMP], index, 1);
            }
            Command::BeginQuery { heap, index } | Command::EndQuery { heap, index } => {
                self.need(c, caps.occlusion, "occlusion")?;
                assert_eq!(
                    self.heap,
                    Some(heap),
                    "{} needs occlusion_heap({heap}) before begin_render",
                    name(c)
                );
                let kinds = [query::OCCLUSION, query::PRECISE];
                self.queries(heap, &kinds, index, 1);
            }
            Command::ResolveQueries {
                heap,
                first,
                count,
                buffer,
                offset,
            } => {
                self.queries(heap, &[], first, count);
                assert!(
                    offset.is_multiple_of(8),
                    "resolve_queries offset {offset} is not a multiple of 8"
                );
                self.range(buffer, offset, count as u64 * 8);
            }
            Command::PushConstants { offset, data } => {
                let end = offset as usize + data.len();
                assert!(
                    offset.is_multiple_of(4) && data.len().is_multiple_of(4),
                    "push_constants offset {offset} and size {} must be multiples of 4",
                    data.len()
                );
                assert!(
                    end <= caps.push as usize,
                    "push_constants ends at {end}, past {} bytes",
                    caps.push
                );
            }
            Command::BeginLabel { .. } => {
                self.need(c, caps.labels, "labels")?;
                self.labels.push(open);
            }
            Command::EndLabel => {
                assert_eq!(
                    self.labels.pop(),
                    Some(open),
                    "end_label must close a label begun in the same scope"
                );
            }
            Command::SetViewports { viewports } => {
                rects(viewports, 24, caps.viewports);
                for v in crate::command::viewports(viewports) {
                    viewport(v.x, v.y, v.width, v.height, v.near, v.far);
                }
            }
            Command::SetViewport {
                x,
                y,
                width,
                height,
                near,
                far,
            } => viewport(x, y, width, height, near, far),
            Command::SetScissors { scissors } => {
                rects(scissors, 16, caps.viewports);
                for r in crate::command::scissors(scissors) {
                    let end = (r.x as u64 + r.width as u64).max(r.y as u64 + r.height as u64);
                    assert!(end <= i32::MAX as u64, "scissor {r:?} is too large");
                }
            }
            Command::Callback { index } => {
                assert!(
                    (index as usize) < self.natives,
                    "callback {index} was recorded without its closure"
                );
            }
            Command::ExecuteIndirect {
                indirect,
                first,
                count,
            } => {
                let size = self.list(c, indirect)?;
                assert!(
                    first as u64 + count as u64 <= size as u64,
                    "{c} draws {first}..{} past indirect buffer {indirect} of {size}",
                    first as u64 + count as u64
                );
                if count > 1 {
                    self.need(c, caps.multi_draw, "multi_draw")?;
                }
            }
            Command::ExecuteIndirectCount {
                indirect,
                buffer,
                offset,
            } => {
                self.list(c, indirect)?;
                assert!(
                    offset.is_multiple_of(4),
                    "{c} range offset {offset} is not a multiple of 4"
                );
                self.range(buffer, offset, 8);
                self.need(c, caps.indirect_count, "indirect_count")?;
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::BuildAccel {
                accel,
                scratch,
                scratch_offset,
                update,
            } => {
                self.need(c, caps.ray, "ray")?;
                let (build, refit) = *self.table.accels.get(accel);
                assert!(
                    self.table.geometry.get(accel).1,
                    "accel {accel} builds from a buffer that was destroyed"
                );
                let size = if update != 0 { refit } else { build };
                assert!(size > 0, "accel {accel} is compacted and cannot be built");
                assert!(
                    scratch_offset.is_multiple_of(256),
                    "build_accel scratch offset {scratch_offset} is not a multiple of 256"
                );
                self.range(scratch, scratch_offset, size);
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::CopyAccel { src, dst, .. } => {
                self.need(c, caps.ray, "ray")?;
                self.table.accels.get(src);
                self.table.accels.get(dst);
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::WriteCompactedSize { accel, heap, index } => {
                self.need(c, caps.ray, "ray")?;
                self.table.accels.get(accel);
                self.queries(heap, &[query::COMPACTED], index, 1);
            }
            #[cfg(any(feature = "vk", feature = "gl"))]
            Command::SetShadingRate {
                width,
                height,
                primitive,
                attachment,
            } => {
                self.need(c, caps.shading_rate, "shading_rate")?;
                assert!(
                    matches!(width, 1 | 2 | 4) && matches!(height, 1 | 2 | 4),
                    "shading rate {width}x{height} is not 1, 2 or 4 on each side"
                );
                for op in [primitive, attachment] {
                    known("combiner", op, crate::combiner::valid(op));
                }
            }
            #[cfg(any(feature = "vk", feature = "gl"))]
            Command::RateAttachment {
                texture, layout: l, ..
            } => {
                self.need(c, caps.rate_images, "rate_images")?;
                known("layout", l, layout::valid(l));
                let t = self.texture(texture, usage::RATE, "RATE");
                assert_eq!(
                    t.format,
                    format::R8_UINT,
                    "rate_attachment texture {texture} must be R8_UINT"
                );
            }
            _ => {}
        }
        Ok(())
    }
}
