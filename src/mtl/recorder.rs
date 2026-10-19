mod applied;
mod textures;

use crate::Buffer;
use crate::Error;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::render::Render;
use crate::mtl::resources::Resources;
use applied::Applied;

use crate::command::Command;
use crate::mtl::device::resource;
use crate::{DynamicState, Format, IndexType, LoadOp, Stage, StoreOp};
use crate::{sets, slot};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSRange, NSString};
use objc2_metal::{
    MTL4ArgumentTable, MTL4BufferRange, MTL4CommandBuffer, MTL4CommandEncoder,
    MTL4ComputeCommandEncoder, MTL4RenderCommandEncoder, MTL4RenderPassDescriptor,
    MTL4TimestampGranularity, MTL4VisibilityOptions, MTLBlitOption, MTLBuffer, MTLClearColor,
    MTLDepthClipMode, MTLIndexType, MTLLoadAction, MTLOrigin, MTLPrimitiveType, MTLRenderStages,
    MTLScissorRect, MTLSize, MTLStages, MTLStoreAction, MTLTexture, MTLTriangleFillMode,
    MTLViewport, MTLVisibilityResultMode,
};
use std::ptr::NonNull;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Owned,
    Buffer,
    Render,
    Compute,
}

pub struct Recorder<'a> {
    buffer: Option<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>,
    table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
    res: &'a Resources,
    kind: Kind,
    ended: bool,
    pub compute: Option<Retained<ProtocolObject<dyn MTL4ComputeCommandEncoder>>>,
    pub render: Option<Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>>,
    pass: Retained<MTL4RenderPassDescriptor>,
    compute_pipeline: Option<&'a Pipeline>,
    render_pipeline: Option<&'a Render>,
    width: u32,
    height: u32,
    primitive: MTLPrimitiveType,
    raster: Option<(MTLTriangleFillMode, MTLDepthClipMode)>,
    applied: Applied,
    index: Option<(u64, usize, MTLIndexType, u64)>,
    tile: MTLSize,
    shared: usize,
}

unsafe impl Send for Recorder<'_> {}

const COMPUTE: MTLStages = MTLStages(MTLStages::Dispatch.0 | MTLStages::Blit.0);

pub const VERTICES: usize = 30;

fn mtl_stages(bits: Stage) -> MTLStages {
    let same =
        Stage::VERTEX | Stage::FRAGMENT | Stage::TASK | Stage::MESH | Stage::COMPUTE | Stage::COPY;
    let mut s = bits & same;
    if bits.contains(Stage::INDIRECT) {
        s |= Stage::VERTEX | Stage::TASK | Stage::MESH | Stage::COMPUTE;
    }
    if !(bits & (Stage::DEPTH | Stage::COLOR)).is_empty() {
        s |= Stage::FRAGMENT;
    }
    let tile = if s.contains(Stage::FRAGMENT) {
        MTLStages::Tile.0
    } else {
        0
    };
    MTLStages(s.0 as usize | tile)
}

fn wait(encoder: &ProtocolObject<dyn MTL4CommandEncoder>, src: Stage, dst: Stage) {
    if !src.is_empty() {
        encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(
            mtl_stages(src),
            mtl_stages(dst),
            MTL4VisibilityOptions::Device,
        );
    }
}

fn mtl_render_stage(bits: crate::Stage) -> MTLRenderStages {
    match bits {
        Stage::VERTEX | Stage::INDIRECT => MTLRenderStages::Vertex,
        Stage::TASK => MTLRenderStages::Object,
        Stage::MESH => MTLRenderStages::Mesh,
        _ => MTLRenderStages::Fragment,
    }
}

fn origin(x: u32, y: u32, z: u32) -> MTLOrigin {
    MTLOrigin {
        x: x as usize,
        y: y as usize,
        z: z as usize,
    }
}

fn size(x: u32, y: u32, z: u32) -> MTLSize {
    MTLSize {
        width: x as usize,
        height: y as usize,
        depth: z as usize,
    }
}

fn row_pitch(
    format: Format,
    width: u32,
    length: u32,
    height: u32,
    count: u32,
) -> (usize, usize, MTLBlitOption) {
    let [bytes, w, h] = format.block();
    let (bytes, option) = match format {
        Format::D24UnormS8 | Format::D32FloatS8 => (4, MTLBlitOption::DepthFromDepthStencil),
        _ => (bytes, MTLBlitOption::None),
    };
    let length = if length == 0 { width } else { length };
    let count = if count == 0 { height } else { count };
    let row = length.div_ceil(w) as usize * bytes as usize;
    (row, count.div_ceil(h) as usize * row, option)
}

fn mtl_load_action(l: crate::LoadOp) -> MTLLoadAction {
    match l {
        LoadOp::Load => MTLLoadAction::Load,
        LoadOp::Clear => MTLLoadAction::Clear,
        LoadOp::DontCare => MTLLoadAction::DontCare,
    }
}

fn mtl_store_action(s: StoreOp, resolve: bool) -> MTLStoreAction {
    match (s, resolve) {
        (StoreOp::Store, false) => MTLStoreAction::Store,
        (StoreOp::Store, true) => MTLStoreAction::StoreAndMultisampleResolve,
        (StoreOp::DontCare, true) => MTLStoreAction::MultisampleResolve,
        (StoreOp::DontCare, false) => MTLStoreAction::DontCare,
    }
}

impl<'a> Recorder<'a> {
    pub fn clear_pass(&mut self) {
        self.pass = MTL4RenderPassDescriptor::new();
    }

    pub fn new(
        table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
        res: &'a Resources,
        buffer: Option<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>,
        kind: Kind,
    ) -> Self {
        Self {
            buffer,
            table,
            res,
            kind,
            ended: false,
            compute: None,
            render: None,
            pass: MTL4RenderPassDescriptor::new(),
            compute_pipeline: None,
            render_pipeline: None,
            width: 0,
            height: 0,
            primitive: MTLPrimitiveType::Triangle,
            raster: None,
            applied: Applied::default(),
            index: None,
            tile: MTLSize {
                width: 0,
                height: 0,
                depth: 1,
            },
            shared: 0,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    fn command_buffer(&self) -> &ProtocolObject<dyn MTL4CommandBuffer> {
        self.buffer
            .as_deref()
            .expect("an imported encoder has no command buffer")
    }

    pub fn raw(&self) -> &ProtocolObject<dyn MTL4CommandBuffer> {
        self.command_buffer()
    }

    pub fn raw_compute(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.compute_encoder()
    }

    pub fn raw_render(&self) -> &ProtocolObject<dyn MTL4RenderCommandEncoder> {
        self.render_encoder()
    }

    pub fn import_render(
        &mut self,
        encoder: Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>,
        width: u32,
        height: u32,
    ) {
        self.bind_argument_table(&encoder);
        self.width = width;
        self.height = height;
        self.raster = None;
        self.applied = Applied::default();
        self.render = Some(encoder);
    }

    pub fn import_compute(
        &mut self,
        encoder: Retained<ProtocolObject<dyn MTL4ComputeCommandEncoder>>,
    ) {
        encoder.setArgumentTable(Some(self.table));
        self.compute = Some(encoder);
    }

    pub fn end(&mut self) -> Result<(), Error> {
        if self.kind == Kind::Owned {
            self.ended = true;
            self.buffer.as_deref().unwrap().endCommandBuffer();
        }
        Ok(())
    }

    fn compute_encoder(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.compute.as_deref().expect("checked by Command::enter")
    }

    fn states(&mut self) -> (&ProtocolObject<dyn MTL4RenderCommandEncoder>, &mut Applied) {
        let e = self.render.as_deref().expect("checked by Command::enter");
        (e, &mut self.applied)
    }

    fn render_encoder(&self) -> &ProtocolObject<dyn MTL4RenderCommandEncoder> {
        self.render.as_deref().expect("checked by Command::enter")
    }

    fn bind_argument_table(&self, encoder: &ProtocolObject<dyn MTL4RenderCommandEncoder>) {
        let all = MTLRenderStages::Vertex
            | MTLRenderStages::Fragment
            | MTLRenderStages::Tile
            | MTLRenderStages::Object
            | MTLRenderStages::Mesh;
        encoder.setArgumentTable_atStages(self.table, all);
    }

    fn index_binding(
        &self,
        buffer: Buffer,
        offset: u64,
        kind: crate::IndexType,
    ) -> (u64, usize, MTLIndexType, u64) {
        let b = self.res.buffers.get(buffer);
        let (kind, size) = match kind {
            IndexType::U16 => (MTLIndexType::UInt16, 2),
            IndexType::U32 => (MTLIndexType::UInt32, 4),
        };
        (
            b.address + offset,
            b.length.saturating_sub(offset as usize) / size as usize * size as usize,
            kind,
            size,
        )
    }

    fn clamp_scissor(&self, x: u32, y: u32, width: u32, height: u32) -> MTLScissorRect {
        let x = x.min(self.width);
        let y = y.min(self.height);
        MTLScissorRect {
            x: x as usize,
            y: y as usize,
            width: width.min(self.width - x) as usize,
            height: height.min(self.height - y) as usize,
        }
    }

    fn bind_set(&self, set: crate::DescriptorSet, offsets: &[u32]) {
        let s = self.res.descriptor_sets.get(set);
        let moves = &self.res.descriptor_pools.get(s.pool).offsets;
        for (i, v) in s.values.iter().enumerate() {
            let mut v = v.load(Ordering::Relaxed);
            if v == 0 {
                continue;
            }
            if let Some(k) = moves[i] {
                v += offsets[k] as u64;
            }
            unsafe {
                match s.set {
                    sets::BUFFERS => self.table.setAddress_atIndex(v, i),
                    sets::SAMPLERS => self.table.setSamplerState_atIndex(resource(v), i),
                    _ => self.table.setTexture_atIndex(resource(v), i),
                }
            }
        }
    }

    pub fn step(&mut self, command: Command) -> Result<(), Error> {
        match command {
            Command::BeginComputePass {
                src_stage,
                dst_stage,
            } => {
                let encoder = self
                    .command_buffer()
                    .computeCommandEncoder()
                    .ok_or_else(|| {
                        Error::OutOfMemory("compute encoder allocation failed".into())
                    })?;
                encoder.setArgumentTable(Some(self.table));
                wait(ProtocolObject::from_ref(&*encoder), src_stage, dst_stage);
                self.compute_pipeline = None;
                self.compute = Some(encoder);
            }
            Command::EndComputePass => self.compute.take().unwrap().endEncoding(),
            Command::BindComputePipeline { pipeline } => {
                let p = self.res.pipelines.get(pipeline);
                self.compute_encoder().setComputePipelineState(&p.state);
                self.compute_pipeline = Some(p);
            }
            Command::Dispatch { x, y, z } => {
                let Some(p) = self.compute_pipeline else {
                    return Ok(());
                };
                if [x, y, z].contains(&0) {
                    return Ok(());
                }
                self.compute_encoder()
                    .dispatchThreadgroups_threadsPerThreadgroup(size(x, y, z), p.group);
            }
            Command::DispatchIndirect { buffer, offset } => {
                let Some(p) = self.compute_pipeline else {
                    return Ok(());
                };
                let address = self.res.buffers.get(buffer).address + offset;
                unsafe {
                    self.compute_encoder()
                        .dispatchThreadgroupsWithIndirectBuffer_threadsPerThreadgroup(
                            address, p.group,
                        )
                };
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => unsafe {
                self.compute_encoder()
                    .copyFromBuffer_sourceOffset_toBuffer_destinationOffset_size(
                        &self.res.buffers.get(src).raw,
                        src_offset as usize,
                        &self.res.buffers.get(dst).raw,
                        dst_offset as usize,
                        size as usize,
                    )
            },
            Command::FillBuffer {
                buffer,
                offset,
                size,
                value,
            } => {
                let b = self.res.buffers.get(buffer);
                let size = if size == !0 {
                    b.length - offset as usize
                } else {
                    size as usize
                };
                unsafe {
                    self.compute_encoder().fillBuffer_range_value(
                        &b.raw,
                        NSRange::new(offset as usize, size),
                        value,
                    )
                }
            }
            Command::MemoryBarrier {
                src_stage,
                dst_stage,
            } => {
                let from = mtl_stages(src_stage) & COMPUTE;
                let to = mtl_stages(dst_stage) & COMPUTE;
                if !from.is_empty() && !to.is_empty() {
                    self.compute_encoder()
                        .barrierAfterEncoderStages_beforeEncoderStages_visibilityOptions(
                            from,
                            to,
                            MTL4VisibilityOptions::Device,
                        );
                }
            }
            Command::DiscardTextures { .. } | Command::PresentBarrier { .. } => {}
            Command::CopyBufferToTexture {
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
                width,
                height,
                depth,
                ..
            } => {
                let t = self.res.textures.get(texture);
                let (row, image, option) =
                    row_pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute_encoder()
                        .copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin_options(
                            &self.res.buffers.get(buffer).raw,
                            offset as usize,
                            row,
                            image,
                            size(width, height, depth),
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                            option,
                        )
                }
            }
            Command::CopyTextureToBuffer {
                texture,
                mip,
                layer,
                x,
                y,
                z,
                width,
                height,
                depth,
                buffer,
                offset,
                row_length,
                row_count,
                ..
            } => {
                let t = self.res.textures.get(texture);
                let (row, image, option) =
                    row_pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute_encoder()
                        .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage_options(
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                            size(width, height, depth),
                            &self.res.buffers.get(buffer).raw,
                            offset as usize,
                            row,
                            image,
                            option,
                        )
                }
            }
            Command::CopyTexture { .. } => self.copy_texture(command)?,
            Command::ColorAttachment {
                index,
                texture,
                load: l,
                store: s,
                clear: [r, g, b, a],
                resolve,
                ..
            } => {
                let res = self.res;
                let t = res.textures.get(texture);
                let attachment = unsafe {
                    self.pass
                        .colorAttachments()
                        .objectAtIndexedSubscript(index as usize)
                };
                attachment.setTexture(Some(&t.raw));
                unsafe { self.pass.setDefaultRasterSampleCount(t.raw.sampleCount()) };
                attachment.setLoadAction(mtl_load_action(l));
                attachment.setStoreAction(mtl_store_action(s, resolve.is_some()));
                attachment.setClearColor(MTLClearColor {
                    red: r as f64,
                    green: g as f64,
                    blue: b as f64,
                    alpha: a as f64,
                });
                if let Some(resolve) = resolve {
                    attachment.setResolveTexture(Some(&res.textures.get(resolve).raw));
                }
            }
            Command::DepthAttachment {
                texture,
                load: l,
                store: s,
                clear_depth: depth,
                stencil_load,
                stencil_store,
                clear_stencil: stencil,
                ..
            } => {
                let t = self.res.textures.get(texture);
                unsafe { self.pass.setDefaultRasterSampleCount(t.raw.sampleCount()) };
                if t.format != Format::S8Uint {
                    let attachment = self.pass.depthAttachment();
                    attachment.setTexture(Some(&t.raw));
                    attachment.setLoadAction(mtl_load_action(l));
                    attachment.setStoreAction(mtl_store_action(s, false));
                    attachment.setClearDepth(depth as f64);
                }
                if matches!(
                    t.format,
                    Format::D24UnormS8 | Format::D32FloatS8 | Format::S8Uint
                ) {
                    let attachment = self.pass.stencilAttachment();
                    attachment.setTexture(Some(&t.raw));
                    attachment.setLoadAction(mtl_load_action(stencil_load));
                    attachment.setStoreAction(mtl_store_action(stencil_store, false));
                    attachment.setClearStencil(stencil);
                }
            }
            Command::BeginRenderPass {
                width,
                height,
                layers,
                src_stage,
                dst_stage,
            } => {
                self.pass.setRenderTargetWidth(width as usize);
                self.pass.setRenderTargetHeight(height as usize);
                if self.pass.defaultRasterSampleCount() == 0 {
                    unsafe { self.pass.setDefaultRasterSampleCount(1) };
                }
                self.pass.setRenderTargetArrayLength(layers as usize);
                let encoder = self
                    .command_buffer()
                    .renderCommandEncoderWithDescriptor(&self.pass)
                    .ok_or_else(|| Error::OutOfMemory("render encoder allocation failed".into()))?;
                self.bind_argument_table(&encoder);
                wait(ProtocolObject::from_ref(&*encoder), src_stage, dst_stage);
                self.render = Some(encoder);
                self.raster = Some((MTLTriangleFillMode::Fill, MTLDepthClipMode::Clip));
                self.applied = Applied::fresh();
                self.clear_pass();
                self.width = width;
                self.height = height;
                self.render_pipeline = None;
            }
            Command::EndRenderPass => self.render.take().unwrap().endEncoding(),
            Command::BindRenderPipeline { pipeline } => {
                let p = self.res.renders.get(pipeline);
                let e = self.render.as_deref().expect("checked by Command::enter");
                e.setRenderPipelineState(&p.state);
                let (fill, clip) = self.raster.unzip();
                if fill != Some(p.fill) {
                    e.setTriangleFillMode(p.fill);
                }
                if clip != Some(p.clip) {
                    e.setDepthClipMode(p.clip);
                }
                self.raster = Some((p.fill, p.clip));
                if !p.states.dynamic.contains(DynamicState::TOPOLOGY) {
                    self.primitive = p.primitive;
                }
                self.applied.bind(e, p);
                self.render_pipeline = Some(p);
            }
            Command::SetCullMode { cull } => {
                let (e, applied) = self.states();
                applied.cull(e, cull);
            }
            Command::SetFrontFace { face } => {
                let (e, applied) = self.states();
                applied.face(e, face);
            }
            Command::SetPrimitiveTopology { topology } => {
                self.primitive = MTLPrimitiveType(topology as usize)
            }
            Command::SetDepthState { state } => {
                let state = self.res.depths.get(state);
                let (e, applied) = self.states();
                applied.depth(e, state);
            }
            Command::SetStencilReference { front, back } => {
                let (e, applied) = self.states();
                applied.reference(e, front, back);
            }
            Command::SetDepthBias {
                constant,
                slope,
                clamp,
            } => {
                let (e, applied) = self.states();
                applied.bias(e, constant, slope, clamp);
            }
            Command::SetBlendColor { color } => {
                let (e, applied) = self.states();
                applied.blend(e, color);
            }
            Command::UseResidencySet { set } => {
                let raw = self.res.sets.get(set);
                self.command_buffer().useResidencySet(raw);
            }
            Command::BindDescriptorSet { set } => self.bind_set(set, &[]),
            Command::BindDescriptorSetDynamic { set, offsets } => self.bind_set(set, offsets),
            Command::BindVertexBuffer {
                slot,
                buffer,
                offset,
            } => {
                let address = self.res.buffers.get(buffer).address + offset;
                unsafe {
                    self.table
                        .setAddress_atIndex(address, VERTICES - slot as usize)
                };
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => self.index = Some(self.index_binding(buffer, offset, kind)),
            Command::Draw {
                vertices,
                instances,
                first_vertex,
                first_instance,
            } => {
                if vertices == 0 || instances == 0 {
                    return Ok(());
                }
                unsafe {
                    self.render_encoder()
                        .drawPrimitives_vertexStart_vertexCount_instanceCount_baseInstance(
                            self.primitive,
                            first_vertex as usize,
                            vertices as usize,
                            instances as usize,
                            first_instance as usize,
                        )
                };
            }
            Command::DrawIndexed {
                indices,
                instances,
                first_index,
                base_vertex,
                first_instance,
            } => {
                let (address, length, kind, size) = self.index.expect("bind_index_buffer");
                if indices == 0 || instances == 0 {
                    return Ok(());
                }
                let skip = first_index as u64 * size;
                unsafe {
                    self.render_encoder()
                        .drawIndexedPrimitives_indexCount_indexType_indexBuffer_indexBufferLength_instanceCount_baseVertex_baseInstance(
                            self.primitive,
                            indices as usize,
                            kind,
                            address + skip,
                            length.saturating_sub(skip as usize),
                            instances as usize,
                            base_vertex as isize,
                            first_instance as usize,
                        )
                };
            }
            Command::DrawIndirect { buffer, offset, .. } => {
                let address = self.res.buffers.get(buffer).address + offset;
                self.render_encoder()
                    .drawPrimitives_indirectBuffer(self.primitive, address);
            }
            Command::DrawIndexedIndirect { buffer, offset, .. } => {
                let (index, length, kind, _) = self.index.expect("bind_index_buffer");
                let address = self.res.buffers.get(buffer).address + offset;
                unsafe {
                    self.render_encoder()
                        .drawIndexedPrimitives_indexType_indexBuffer_indexBufferLength_indirectBuffer(
                            self.primitive,
                            kind,
                            index,
                            length,
                            address,
                        )
                };
            }
            Command::InputAttachmentBarrier => {}
            Command::TileMemory {
                width,
                height,
                shared,
            } => {
                self.pass.setTileWidth(width as usize);
                self.pass.setTileHeight(height as usize);
                self.pass.setThreadgroupMemoryLength(shared as usize);
                self.shared = shared as usize;
                self.tile = MTLSize {
                    width: width as usize,
                    height: height as usize,
                    depth: 1,
                };
            }
            Command::DispatchTile { pipeline } => {
                self.render_encoder()
                    .setRenderPipelineState(self.res.tiles.get(pipeline));
                self.render_pipeline = None;
                if self.shared > 0 {
                    unsafe {
                        self.render_encoder()
                            .setThreadgroupMemoryLength_offset_atIndex(self.shared, 0, 0)
                    };
                }
                self.render_encoder().dispatchThreadsPerTile(self.tile);
            }
            Command::DrawMeshTasks { x, y, z } => {
                let p = self.render_pipeline.expect("bind_render_pipeline");
                if [x, y, z].contains(&0) {
                    return Ok(());
                }
                self.render_encoder()
                    .drawMeshThreadgroups_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                        size(x, y, z),
                        p.task,
                        p.mesh,
                    );
            }
            Command::ExecuteIndirect {
                indirect,
                first,
                count,
            } => {
                if count == 0 {
                    return Ok(());
                }
                let icb = &self.res.indirects.get(indirect).raw;
                let range = NSRange::new(first as usize, count as usize);
                unsafe {
                    self.render_encoder()
                        .executeCommandsInBuffer_withRange(icb, range)
                };
            }
            Command::ExecuteIndirectCount {
                indirect,
                buffer,
                offset,
            } => {
                let icb = &self.res.indirects.get(indirect).raw;
                let range = self.res.buffers.get(buffer).address + offset;
                unsafe {
                    self.render_encoder()
                        .executeCommandsInBuffer_indirectBuffer(icb, range)
                };
            }
            Command::DrawMeshTasksIndirect { buffer, offset, .. } => {
                let p = self.render_pipeline.expect("bind_render_pipeline");
                let address = self.res.buffers.get(buffer).address + offset;
                self.render_encoder()
                    .drawMeshThreadgroupsWithIndirectBuffer_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                        address,
                        p.task,
                        p.mesh,
                    );
            }
            Command::OcclusionPool { pool: heap } => {
                if let Query::Buffer(buffer) = self.res.queries.get(heap) {
                    unsafe { self.pass.setVisibilityResultBuffer(Some(buffer)) };
                }
            }
            Command::ResetQueries { .. } => {}
            Command::WriteTimestamp {
                pool: heap,
                index,
                stage,
            } => {
                let Query::Timestamp(heap) = self.res.queries.get(heap) else {
                    unreachable!()
                };
                let index = index as usize;
                unsafe {
                    match (&self.compute, &self.render) {
                        (Some(e), _) => e.writeTimestampWithGranularity_intoHeap_atIndex(
                            MTL4TimestampGranularity::Precise,
                            heap,
                            index,
                        ),
                        (_, Some(e)) => e
                            .writeTimestampWithGranularity_afterStage_intoHeap_atIndex(
                                MTL4TimestampGranularity::Precise,
                                mtl_render_stage(stage),
                                heap,
                                index,
                            ),
                        _ => self
                            .command_buffer()
                            .writeTimestampIntoHeap_atIndex(heap, index),
                    }
                }
            }
            Command::BeginOcclusionQuery { index, .. } => unsafe {
                self.render_encoder().setVisibilityResultMode_offset(
                    MTLVisibilityResultMode::Counting,
                    index as usize * 8,
                )
            },
            Command::EndOcclusionQuery { index, .. } => unsafe {
                self.render_encoder().setVisibilityResultMode_offset(
                    MTLVisibilityResultMode::Disabled,
                    index as usize * 8,
                )
            },
            Command::ResolveTimestamps {
                pool: heap,
                first,
                count,
                buffer,
                offset,
            }
            | Command::ResolveOcclusion {
                pool: heap,
                first,
                count,
                buffer,
                offset,
            } => {
                if count == 0 {
                    return Ok(());
                }
                let dst = &self.res.buffers.get(buffer).raw;
                match self.res.queries.get(heap) {
                    Query::Timestamp(heap) => unsafe {
                        self.command_buffer()
                            .resolveCounterHeap_withRange_intoBuffer_waitFence_updateFence(
                                heap,
                                NSRange::new(first as usize, count as usize),
                                MTL4BufferRange {
                                    bufferAddress: dst.gpuAddress() + offset,
                                    length: count as u64 * 8,
                                },
                                None,
                                None,
                            )
                    },
                    Query::Buffer(src) => unsafe {
                        self.compute_encoder()
                            .copyFromBuffer_sourceOffset_toBuffer_destinationOffset_size(
                                src,
                                first as usize * 8,
                                dst,
                                offset as usize,
                                count as usize * 8,
                            )
                    },
                }
            }
            Command::SetViewport { viewports } => {
                let mut list = [MTLViewport {
                    originX: 0.0,
                    originY: 0.0,
                    width: 0.0,
                    height: 0.0,
                    znear: 0.0,
                    zfar: 0.0,
                }; slot::VIEWPORTS as usize];
                let mut n = 0;
                for (v, out) in viewports.iter().zip(&mut list) {
                    *out = MTLViewport {
                        originX: v.x as f64,
                        originY: v.y as f64,
                        width: v.width as f64,
                        height: v.height as f64,
                        znear: v.near as f64,
                        zfar: v.far as f64,
                    };
                    n += 1;
                }
                unsafe {
                    self.render_encoder()
                        .setViewports_count(NonNull::new(list.as_mut_ptr()).unwrap(), n)
                }
            }
            Command::SetScissor { scissors } => {
                let mut list = [MTLScissorRect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                }; slot::VIEWPORTS as usize];
                let mut n = 0;
                for (r, out) in scissors.iter().zip(&mut list) {
                    *out = self.clamp_scissor(r.x, r.y, r.width, r.height);
                    n += 1;
                }
                unsafe {
                    self.render_encoder()
                        .setScissorRects_count(NonNull::new(list.as_mut_ptr()).unwrap(), n)
                }
            }
            Command::BeginDebugLabel { text } => {
                let text = NSString::from_str(text);
                match (&self.compute, &self.render) {
                    (Some(e), _) => e.pushDebugGroup(&text),
                    (_, Some(e)) => e.pushDebugGroup(&text),
                    _ => self.command_buffer().pushDebugGroup(&text),
                }
            }
            Command::EndDebugLabel => match (&self.compute, &self.render) {
                (Some(e), _) => e.popDebugGroup(),
                (_, Some(e)) => e.popDebugGroup(),
                _ => self.command_buffer().popDebugGroup(),
            },
        }
        Ok(())
    }
}

impl Drop for Recorder<'_> {
    fn drop(&mut self) {
        if self.kind != Kind::Compute
            && let Some(e) = self.compute.take()
        {
            e.endEncoding();
        }
        if self.kind != Kind::Render
            && let Some(e) = self.render.take()
        {
            e.endEncoding();
        }
        if self.kind == Kind::Owned && !self.ended {
            self.command_buffer().endCommandBuffer();
        }
    }
}

impl crate::CommandBuffer<'_> {
    pub fn raw(&self) -> &ProtocolObject<dyn MTL4CommandBuffer> {
        self.recording.recorder.raw()
    }
}

impl crate::ComputePass<'_, '_> {
    pub fn raw(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.recording.recorder.raw_compute()
    }
}

impl crate::RenderPass<'_, '_> {
    pub fn raw(&self) -> &ProtocolObject<dyn MTL4RenderCommandEncoder> {
        self.recording.recorder.raw_render()
    }
}
