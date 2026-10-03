use crate::table::Table;
use crate::{
    Caps, Command, TextureInfo, access, cull, face, format, index, layout, load, query, slot,
    stage, store, topology, usage,
};

pub struct Check<'a> {
    caps: &'a Caps,
    table: &'a Table,
    natives: usize,
    open: &'static str,
    index: Option<u64>,
    compute: bool,
    render: bool,
    heap: Option<u32>,
    rate: bool,
    targets: Vec<(u32, bool)>,
}

fn known(what: &str, value: u32, valid: bool) {
    assert!(valid, "unknown {what} {value}");
}

fn bits(what: &str, value: u32, all: u32) {
    assert!(value & !all == 0, "unknown {what} bits {:#x}", value & !all);
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
    assert!(count != 0, "{what} count is 0 for texture {texture}");
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
            compute: false,
            render: false,
            heap: None,
            rate: false,
            targets: Vec::new(),
        };
        while !bytes.is_empty() {
            let c = Command::read(&mut bytes)
                .unwrap_or_else(|name| panic!("{name} is not in this backend"));
            check.step(&c)?;
        }
        assert!(
            check.open == "outside",
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

    fn filled(&self, c: &Command, buffer: u32, offset: u64, size: u64) {
        let size = self.range(buffer, offset, size);
        assert!(size > 0, "{} covers 0 bytes", name(c));
    }

    fn barrier(&self, src_stage: u32, src_access: u32, dst_stage: u32, dst_access: u32) {
        bits("stage", src_stage | dst_stage, stage::BITS);
        bits("access", src_access | dst_access, access::BITS);
    }

    fn texture(&self, texture: u32, needs: u32, what: &str) -> &TextureInfo {
        let t = self.table.textures.get(texture);
        assert!(
            t.usage & needs != 0,
            "texture {texture} has no usage::{what}"
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

    fn draw(&self, c: &Command, indexed: bool) {
        assert!(self.render, "{} needs bind_render_pipeline first", name(c));
        assert!(
            !indexed || self.index.is_some(),
            "{} needs bind_index_buffer first",
            name(c)
        );
    }

    fn indirect(&self, buffer: u32, offset: u64, count: u32, stride: u32, size: u64) {
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
        self.range(buffer, offset, bytes * rows.div_ceil(bh) as u64 * d as u64);
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
                assert!(open == "outside", "{c} inside begin_{open}");
                for &(texture, clear) in self.targets.iter().filter(|_| !self.rate) {
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
                self.open = "render";
            }
            Command::BeginCompute => {
                assert!(open == "outside", "{c} inside begin_{open}");
                self.open = "compute";
            }
            Command::EndCompute => {
                assert!(open == "compute", "end_compute without begin_compute");
                self.open = "outside";
            }
            Command::EndRender => {
                assert!(open == "render", "end_render without begin_render");
                self.open = "outside";
                self.targets.clear();
                self.heap = None;
                self.rate = false;
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
                assert!(
                    offset.is_multiple_of(4) && (size == !0 || size.is_multiple_of(4)),
                    "fill_buffer offset {offset} and size {size} must be multiples of 4"
                );
                self.filled(c, buffer, offset, size);
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
                    self.texture(resolve, usage::COLOR, "COLOR");
                }
            }
            Command::DepthAttachment {
                texture,
                layout: l,
                load: ld,
                store: st,
                stencil_load,
                stencil_store,
                ..
            } => {
                known("layout", l, layout::valid(l));
                known("load", ld, load::valid(ld));
                known("store", st, store::valid(st));
                known("load", stencil_load, load::valid(stencil_load));
                known("store", stencil_store, store::valid(stencil_store));
                self.texture(texture, usage::DEPTH, "DEPTH");
                let clear = ld == load::CLEAR || stencil_load == load::CLEAR;
                self.targets.push((texture, clear));
            }
            Command::BindRenderPipeline { pipeline } => {
                self.table.renders.get(pipeline);
                self.render = true;
            }
            Command::SetCull { cull: v } => known("cull", v, cull::valid(v)),
            Command::SetFace { face: v } => known("face", v, face::valid(v)),
            Command::SetTopology { topology: v } => known("topology", v, topology::valid(v)),
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
                slot(i, slot::UNIFORMS);
                aligned(offset, caps.bind_alignment);
                self.filled(c, buffer, offset, size);
            }
            Command::BindStorage {
                slot: i,
                buffer,
                offset,
                size,
            } => {
                self.need(c, caps.storage, "storage")?;
                slot(i, slot::STORAGES);
                aligned(offset, caps.bind_alignment);
                self.filled(c, buffer, offset, size);
            }
            Command::BindTexture { slot: i, texture } => {
                slot(i, slot::TEXTURES);
                self.texture(texture, usage::SAMPLED, "SAMPLED");
            }
            Command::BindImage { slot: i, texture } => {
                self.need(c, caps.storage, "storage")?;
                slot(i, slot::IMAGES);
                self.texture(texture, usage::STORAGE, "STORAGE");
            }
            Command::BindSampler { slot: i, sampler } => {
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
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => {
                known("index kind", kind, index::valid(kind));
                self.range(buffer, offset, 0);
                self.index = Some(offset);
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
                if self.index != Some(0) {
                    self.need(c, caps.indirect_offset, "indirect_offset")?;
                }
                self.indirect(buffer, offset, count, stride, 20);
            }
            Command::DrawMesh { .. } => {
                self.need(c, caps.mesh, "mesh")?;
                self.draw(c, false);
            }
            Command::DrawMeshIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.need(c, caps.mesh, "mesh")?;
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
                assert!(
                    stage.count_ones() == 1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
                bits("stage", stage, stage::BITS);
                self.need(c, caps.timestamps, "timestamps")?;
                self.queries(heap, &[query::TIMESTAMP], index, 1);
            }
            Command::BeginQuery { heap, index } | Command::EndQuery { heap, index } => {
                match self.table.queries.get(heap).0 {
                    query::STATISTICS => self.need(c, caps.statistics, "statistics")?,
                    _ => {
                        self.need(c, caps.occlusion, "occlusion")?;
                        assert!(
                            self.heap == Some(heap),
                            "{} needs occlusion_heap({heap}) before begin_render",
                            name(c)
                        );
                    }
                }
                let kinds = [query::OCCLUSION, query::PRECISE, query::STATISTICS];
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
            Command::BeginLabel { .. } => self.need(c, caps.labels, "labels")?,
            Command::SetViewports { viewports } => rects(viewports, 24, caps.viewports),
            Command::SetScissors { scissors } => {
                rects(scissors, 16, caps.viewports);
                for r in crate::command::scissors(scissors) {
                    let end = (r.x as u64 + r.width as u64).max(r.y as u64 + r.height as u64);
                    assert!(end <= i32::MAX as u64, "scissor {r:?} is too large");
                }
            }
            Command::SetDepthBounds { .. } => self.need(c, caps.depth_bounds, "depth_bounds")?,
            Command::Callback { index } => {
                assert!(
                    (index as usize) < self.natives,
                    "callback {index} was recorded without its closure"
                );
            }
            #[cfg(feature = "mtl")]
            Command::RateMap { .. } => self.rate = true,
            #[cfg(feature = "vk")]
            Command::SetLineWidth { width } => {
                if width != 1.0 {
                    self.need(c, caps.wide_lines, "wide_lines")?;
                }
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::Multiview { views } => {
                if views > 1 {
                    self.need(c, views <= caps.views, "views")?;
                }
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::BuildAccel { .. } | Command::CopyAccel { .. } => {
                self.need(c, caps.ray, "ray")?;
            }
            #[cfg(any(feature = "vk", feature = "mtl"))]
            Command::WriteCompactedSize { heap, index, .. } => {
                self.need(c, caps.ray, "ray")?;
                self.queries(heap, &[query::COMPACTED], index, 1);
            }
            #[cfg(feature = "vk")]
            Command::BeginConditional { buffer, offset, .. } => {
                self.need(c, caps.conditional, "conditional")?;
                self.range(buffer, offset, 4);
            }
            #[cfg(feature = "vk")]
            Command::SetShadingRate { .. } => {
                self.need(c, caps.shading_rate, "shading_rate")?;
            }
            #[cfg(feature = "vk")]
            Command::RateAttachment {
                texture, layout: l, ..
            } => {
                self.need(c, caps.shading_rate, "shading_rate")?;
                known("layout", l, layout::valid(l));
                self.table.textures.get(texture);
            }
            #[cfg(feature = "vk")]
            Command::BindRayPipeline { .. } | Command::TraceRays { .. } => {
                self.need(c, caps.rays, "rays")?;
            }
            #[cfg(feature = "vk")]
            Command::ExecuteGenerated { .. } => self.need(c, caps.generated, "generated")?,
            _ => {}
        }
        Ok(())
    }
}
