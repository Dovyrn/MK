use crate::table::Table;
use crate::{Caps, Command, TextureInfo, format, layout, query, slot, usage};

pub struct Check<'a> {
    caps: &'a Caps,
    table: &'a Table,
    natives: usize,
    open: &'static str,
    index: u64,
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
            index: 0,
        };
        while !bytes.is_empty() {
            check.step(&Command::read(&mut bytes)?)?;
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

    fn range(&self, buffer: u32, offset: u64, size: u64) {
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
        let &(kind, total) = self.table.heaps.get(heap);
        assert!(
            kinds.is_empty() || kinds.contains(&kind),
            "query heap {heap} has kind {kind}, not {kinds:?}"
        );
        if count > 0 {
            let last = first.saturating_add(count - 1);
            assert!(last < total, "query {last} is outside 0..{total}");
        }
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
        span("mip", mip, 1, t.mips, texture);
        span("layer", layer, 1, t.layers, texture);
        let size = [
            (t.width >> mip).max(1),
            (t.height >> mip).max(1),
            (t.depth >> mip).max(1),
        ];
        assert!(
            x + w <= size[0] && y + h <= size[1] && z + d <= size[2],
            "region {x},{y},{z} {w}x{h}x{d} is outside mip {mip} of texture {texture} ({}x{}x{})",
            size[0],
            size[1],
            size[2]
        );
        let [_, bw, bh] = format::block(t.format);
        let row = if row_length == 0 { w } else { row_length };
        let rows = if row_count == 0 { h } else { row_count };
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
            }
            Command::BindPipeline { pipeline } => {
                self.need(c, caps.compute, "compute")?;
                self.table.pipelines.get(pipeline);
            }
            Command::Dispatch { .. } => self.need(c, caps.compute, "compute")?,
            Command::DispatchIndirect { buffer, offset } => {
                self.need(c, caps.compute, "compute")?;
                self.range(buffer, offset, 12);
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => {
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
                self.range(buffer, offset, size);
            }
            Command::TextureBarrier {
                texture,
                base_mip,
                mips,
                base_layer,
                layers,
                ..
            } => {
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
                resolve,
                ..
            } => {
                slot(index, slot::COLORS);
                self.texture(texture, usage::COLOR, "COLOR");
                if resolve != !0 {
                    self.texture(resolve, usage::COLOR, "COLOR");
                }
            }
            Command::DepthAttachment { texture, .. } => {
                self.texture(texture, usage::DEPTH, "DEPTH");
            }
            Command::BindRenderPipeline { pipeline } => {
                self.table.renders.get(pipeline);
            }
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
                self.range(buffer, offset, size);
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
                self.range(buffer, offset, size);
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
            Command::BindIndexBuffer { buffer, offset, .. } => {
                self.range(buffer, offset, 0);
                self.index = offset;
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
                if self.index != 0 {
                    self.need(c, caps.indirect_offset, "indirect_offset")?;
                }
                self.indirect(buffer, offset, count, stride, 20);
            }
            Command::DrawMesh { .. } => self.need(c, caps.mesh, "mesh")?,
            Command::DrawMeshIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.need(c, caps.mesh, "mesh")?;
                self.indirect(buffer, offset, count, stride, 12);
            }
            Command::OcclusionHeap { heap } => {
                self.queries(heap, &[query::OCCLUSION, query::PRECISE], 0, 0);
            }
            Command::ResetQueries { heap, first, count } => {
                self.queries(heap, &[], first, count);
            }
            Command::WriteTimestamp { heap, index, stage } => {
                assert!(
                    stage.count_ones() == 1,
                    "write_timestamp stage {stage:#x} must be one stage"
                );
                self.need(c, caps.timestamps, "timestamps")?;
                self.queries(heap, &[query::TIMESTAMP], index, 1);
            }
            Command::BeginQuery { heap, index } | Command::EndQuery { heap, index } => {
                match self.table.heaps.get(heap).0 {
                    query::STATISTICS => self.need(c, caps.statistics, "statistics")?,
                    _ => self.need(c, caps.occlusion, "occlusion")?,
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
            Command::SetScissors { scissors } => rects(scissors, 16, caps.viewports),
            Command::SetDepthBounds { .. } => self.need(c, caps.depth_bounds, "depth_bounds")?,
            Command::Callback { index } => {
                assert!(
                    (index as usize) < self.natives,
                    "callback {index} was recorded without its closure"
                );
            }
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
            Command::RateAttachment { texture, .. } => {
                self.need(c, caps.shading_rate, "shading_rate")?;
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
