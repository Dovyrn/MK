use crate::mtl::frame::Frame;
use crate::mtl::query::Query;
use crate::mtl::queue::Queue;
use crate::mtl::resources::Resources;
use crate::mtl::upscaler::Upscaler;
use crate::mtl::{Callback, Native};
use crate::{Command, access, face, format, index, load, stage, store};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSRange, NSString};
use objc2_metal::{
    MTL4ArgumentTable, MTL4BufferRange, MTL4CommandBuffer, MTL4CommandEncoder,
    MTL4ComputeCommandEncoder, MTL4RenderCommandEncoder, MTL4RenderPassDescriptor,
    MTL4TimestampGranularity, MTL4VisibilityOptions, MTLBuffer, MTLClearColor, MTLCullMode,
    MTLDevice, MTLFence, MTLGPUFamily, MTLIndexType, MTLLoadAction, MTLOrigin, MTLPrimitiveType,
    MTLRenderStages, MTLResidencySet, MTLScissorRect, MTLSize, MTLStages, MTLStoreAction,
    MTLVertexAmplificationViewMapping, MTLViewport, MTLVisibilityResultMode, MTLWinding,
};
use objc2_metal_fx::{
    MTL4FXSpatialScaler, MTL4FXTemporalScaler, MTLFXSpatialScalerBase, MTLFXTemporalScalerBase,
};
use std::ptr::NonNull;

pub struct Recorder<'a> {
    device: &'a ProtocolObject<dyn MTLDevice>,
    residency: &'a ProtocolObject<dyn MTLResidencySet>,
    buffer: &'a ProtocolObject<dyn MTL4CommandBuffer>,
    table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
    frame: &'a mut Frame,
    res: &'a Resources,
    natives: &'a [Callback],
    fence: &'a ProtocolObject<dyn MTLFence>,
    fenced: bool,
    waiting: bool,
    pub compute: Option<Retained<ProtocolObject<dyn MTL4ComputeCommandEncoder>>>,
    pub render: Option<Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>>,
    pass: Retained<MTL4RenderPassDescriptor>,
    pending: (MTLStages, MTLStages, MTL4VisibilityOptions),
    views: u32,
    group: MTLSize,
    task: MTLSize,
    mesh: MTLSize,
    primitive: MTLPrimitiveType,
    index: (u64, usize, MTLIndexType, u64),
    push: [u8; 256],
    size: usize,
    dirty: bool,
}

const COMPUTE: MTLStages =
    MTLStages(MTLStages::Dispatch.0 | MTLStages::Blit.0 | MTLStages::AccelerationStructure.0);

const RENDER: MTLStages = MTLStages(
    MTLStages::Vertex.0
        | MTLStages::Fragment.0
        | MTLStages::Tile.0
        | MTLStages::Object.0
        | MTLStages::Mesh.0,
);

const NONE: u32 = !0;

fn stages(bits: u32) -> MTLStages {
    let mut s = bits & 0x7c00_001f;
    if bits & stage::INDIRECT != 0 {
        s |= stage::VERTEX | stage::TASK | stage::MESH | stage::COMPUTE;
    }
    if bits & (stage::DEPTH | stage::COLOR) != 0 {
        s |= stage::FRAGMENT;
    }
    MTLStages(s as usize)
}

fn render_stage(bits: u32) -> MTLRenderStages {
    match bits {
        stage::VERTEX => MTLRenderStages::Vertex,
        stage::TILE => MTLRenderStages::Tile,
        stage::TASK => MTLRenderStages::Object,
        stage::MESH => MTLRenderStages::Mesh,
        stage::FRAGMENT => MTLRenderStages::Fragment,
        s => panic!("timestamp stage {s} is not one render stage"),
    }
}

fn visibility(src: u32, dst: u32) -> MTL4VisibilityOptions {
    if (src | dst) & (access::HOST_READ | access::HOST_WRITE) != 0 {
        MTL4VisibilityOptions::Device
    } else {
        MTL4VisibilityOptions::None
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

fn pitch(format: u32, width: u32, length: u32, height: u32, count: u32) -> (usize, usize) {
    let [bytes, w, h] = format::block(format);
    let length = if length == 0 { width } else { length };
    let count = if count == 0 { height } else { count };
    let row = length.div_ceil(w) * bytes;
    (row as usize, (count.div_ceil(h) * row) as usize)
}

fn load(l: u32) -> MTLLoadAction {
    match l {
        load::LOAD => MTLLoadAction::Load,
        load::CLEAR => MTLLoadAction::Clear,
        load::DONT_CARE => MTLLoadAction::DontCare,
        l => panic!("unknown load {l}"),
    }
}

fn store(s: u32, resolve: bool) -> MTLStoreAction {
    match (s, resolve) {
        (store::STORE, false) => MTLStoreAction::Store,
        (store::STORE, true) => MTLStoreAction::StoreAndMultisampleResolve,
        (store::DONT_CARE, true) => MTLStoreAction::MultisampleResolve,
        (store::DONT_CARE, false) => MTLStoreAction::DontCare,
        (s, _) => panic!("unknown store {s}"),
    }
}

impl<'a> Recorder<'a> {
    pub fn new(
        device: &'a ProtocolObject<dyn MTLDevice>,
        queue: &'a Queue,
        frame: &'a mut Frame,
        res: &'a Resources,
        natives: &'a [Callback],
    ) -> Self {
        Self {
            device,
            residency: &queue.residency,
            buffer: &queue.buffer,
            table: &queue.table,
            frame,
            res,
            natives,
            fence: &queue.fence,
            fenced: false,
            waiting: false,
            compute: None,
            render: None,
            pass: MTL4RenderPassDescriptor::new(),
            pending: (MTLStages(0), MTLStages(0), MTL4VisibilityOptions::None),
            views: 0,
            group: size(1, 1, 1),
            task: size(1, 1, 1),
            mesh: size(1, 1, 1),
            primitive: MTLPrimitiveType::Triangle,
            index: (0, 0, MTLIndexType::UInt16, 2),
            push: [0; 256],
            size: 0,
            dirty: false,
        }
    }

    fn compute(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.compute.as_deref().expect("checked by Command::enter")
    }

    fn render(&self) -> &ProtocolObject<dyn MTL4RenderCommandEncoder> {
        self.render.as_deref().expect("checked by Command::enter")
    }

    fn encoder(&self) -> Option<(&ProtocolObject<dyn MTL4CommandEncoder>, MTLStages)> {
        match (&self.compute, &self.render) {
            (Some(c), _) => Some((ProtocolObject::from_ref(&**c), COMPUTE)),
            (_, Some(r)) => Some((ProtocolObject::from_ref(&**r), RENDER)),
            _ => None,
        }
    }

    fn ended(&self, encoder: &ProtocolObject<dyn MTL4CommandEncoder>, mask: MTLStages) {
        if self.fenced {
            encoder.updateFence_afterEncoderStages(self.fence, mask);
        }
        encoder.endEncoding();
    }

    fn opened(&mut self, encoder: &ProtocolObject<dyn MTL4CommandEncoder>, mask: MTLStages) {
        if self.waiting {
            encoder.waitForFence_beforeEncoderStages(self.fence, mask);
            self.waiting = false;
        }
        let (after, before, options) = self.pending;
        let now = before & mask;
        if !after.is_empty() && !now.is_empty() {
            encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(after, now, options);
        }
        let rest = MTLStages(before.0 & !mask.0);
        self.pending = if rest.is_empty() {
            (MTLStages(0), MTLStages(0), MTL4VisibilityOptions::None)
        } else {
            (after, rest, options)
        };
    }

    fn barrier(&mut self, queue: bool, src: (u32, u32), dst: (u32, u32)) {
        let (from, to) = (stages(src.0), stages(dst.0));
        let options = visibility(src.1, dst.1);
        let Some((encoder, mask)) = self.encoder() else {
            self.pending.0 |= from;
            self.pending.1 |= to;
            self.pending.2 |= options;
            return;
        };
        let inside = from & mask;
        let outside = if queue {
            MTLStages::All
        } else {
            MTLStages(from.0 & !mask.0)
        };
        let before = to & mask;
        let later = MTLStages(to.0 & !mask.0);
        if !before.is_empty() && !inside.is_empty() {
            encoder.barrierAfterEncoderStages_beforeEncoderStages_visibilityOptions(
                inside, before, options,
            );
        }
        if !before.is_empty() && !outside.is_empty() {
            encoder
                .barrierAfterQueueStages_beforeStages_visibilityOptions(outside, before, options);
        }
        if !later.is_empty() && !inside.is_empty() {
            encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(inside, later, options);
        }
        if !later.is_empty() && !outside.is_empty() {
            self.pending.0 |= outside;
            self.pending.1 |= later;
            self.pending.2 |= options;
        }
    }

    fn flush(&mut self) {
        if self.dirty {
            let address = self
                .frame
                .push(self.device, self.residency, &self.push[..self.size]);
            unsafe { self.table.setAddress_atIndex(address, 0) };
            self.dirty = false;
        }
    }

    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), &'static str> {
        let mut scan = bytes;
        while !scan.is_empty() {
            if let Command::Upscale { .. } = Command::read(&mut scan)? {
                self.fenced = true;
                break;
            }
        }
        let mut open = "";
        while !bytes.is_empty() {
            let command = Command::read(&mut bytes)?;
            open = command.enter(open);
            self.step(command)?;
        }
        assert!(open.is_empty(), "stream ends inside begin_{open}");
        Ok(())
    }

    fn step(&mut self, command: Command) -> Result<(), &'static str> {
        match command {
            Command::BeginCompute => {
                let encoder = self
                    .buffer
                    .computeCommandEncoder()
                    .ok_or("compute encoder creation failed")?;
                encoder.setArgumentTable(Some(self.table));
                self.opened(ProtocolObject::from_ref(&*encoder), COMPUTE);
                self.compute = Some(encoder);
            }
            Command::EndCompute => {
                let encoder = self.compute.take().unwrap();
                self.ended(ProtocolObject::from_ref(&*encoder), COMPUTE);
            }
            Command::BindPipeline { pipeline } => {
                let p = self.res.pipelines.get(pipeline);
                self.compute().setComputePipelineState(&p.state);
                self.group = p.group;
            }
            Command::PushConstants { offset, data } => {
                let at = offset as usize;
                self.push[at..at + data.len()].copy_from_slice(data);
                self.size = self.size.max(at + data.len());
                self.dirty = true;
            }
            Command::Dispatch { x, y, z } => {
                self.flush();
                self.compute()
                    .dispatchThreadgroups_threadsPerThreadgroup(size(x, y, z), self.group);
            }
            Command::DispatchIndirect { buffer, offset } => {
                self.flush();
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                unsafe {
                    self.compute()
                        .dispatchThreadgroupsWithIndirectBuffer_threadsPerThreadgroup(
                            address, self.group,
                        )
                };
            }
            Command::DispatchThreads { x, y, z } => {
                self.flush();
                self.compute()
                    .dispatchThreads_threadsPerThreadgroup(size(x, y, z), self.group);
            }
            Command::CopyBuffer {
                src,
                src_offset,
                dst,
                dst_offset,
                size,
            } => unsafe {
                self.compute()
                    .copyFromBuffer_sourceOffset_toBuffer_destinationOffset_size(
                        self.res.buffers.get(src),
                        src_offset as usize,
                        self.res.buffers.get(dst),
                        dst_offset as usize,
                        size as usize,
                    )
            },
            Command::FillBuffer {
                buffer,
                offset,
                size,
                value,
            } => unsafe {
                self.compute().fillBuffer_range_value(
                    self.res.buffers.get(buffer),
                    NSRange::new(offset as usize, size as usize),
                    value,
                )
            },
            Command::Barrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            }
            | Command::TextureBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
                ..
            } => self.barrier(false, (src_stage, src_access), (dst_stage, dst_access)),
            Command::QueueBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            } => self.barrier(true, (src_stage, src_access), (dst_stage, dst_access)),
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
                let (row, image) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute()
                        .copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                            self.res.buffers.get(buffer),
                            offset as usize,
                            row,
                            image,
                            size(width, height, depth),
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
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
                let (row, image) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute()
                        .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                            size(width, height, depth),
                            self.res.buffers.get(buffer),
                            offset as usize,
                            row,
                            image,
                        )
                }
            }
            Command::ColorAttachment {
                index,
                texture,
                load: l,
                store: s,
                r,
                g,
                b,
                a,
                resolve,
                ..
            } => {
                let attachment = unsafe {
                    self.pass
                        .colorAttachments()
                        .objectAtIndexedSubscript(index as usize)
                };
                attachment.setTexture(Some(&self.res.textures.get(texture).raw));
                attachment.setLoadAction(load(l));
                attachment.setStoreAction(store(s, resolve != NONE));
                attachment.setClearColor(MTLClearColor {
                    red: r as f64,
                    green: g as f64,
                    blue: b as f64,
                    alpha: a as f64,
                });
                if resolve != NONE {
                    attachment.setResolveTexture(Some(&self.res.textures.get(resolve).raw));
                }
            }
            Command::DepthAttachment {
                texture,
                load: l,
                store: s,
                depth,
                stencil_load,
                stencil_store,
                stencil,
                ..
            } => {
                let t = self.res.textures.get(texture);
                let attachment = self.pass.depthAttachment();
                attachment.setTexture(Some(&t.raw));
                attachment.setLoadAction(load(l));
                attachment.setStoreAction(store(s, false));
                attachment.setClearDepth(depth as f64);
                if matches!(
                    t.format,
                    format::D24_UNORM_S8 | format::D32_FLOAT_S8 | format::S8_UINT
                ) {
                    let attachment = self.pass.stencilAttachment();
                    attachment.setTexture(Some(&t.raw));
                    attachment.setLoadAction(load(stencil_load));
                    attachment.setStoreAction(store(stencil_store, false));
                    attachment.setClearStencil(stencil);
                }
            }
            Command::BeginRender {
                width,
                height,
                layers,
            } => {
                self.pass.setRenderTargetWidth(width as usize);
                self.pass.setRenderTargetHeight(height as usize);
                self.pass.setRenderTargetArrayLength(layers as usize);
                let encoder = self
                    .buffer
                    .renderCommandEncoderWithDescriptor(&self.pass)
                    .ok_or("render encoder creation failed")?;
                let all = MTLRenderStages::Vertex
                    | MTLRenderStages::Fragment
                    | MTLRenderStages::Object
                    | MTLRenderStages::Mesh;
                encoder.setArgumentTable_atStages(self.table, all);
                encoder.setFrontFacingWinding(MTLWinding::CounterClockwise);
                if self.views > 1 {
                    let mappings: Vec<_> = (0..self.views)
                        .map(|i| MTLVertexAmplificationViewMapping {
                            viewportArrayIndexOffset: 0,
                            renderTargetArrayIndexOffset: i,
                        })
                        .collect();
                    unsafe {
                        encoder.setVertexAmplificationCount_viewMappings(
                            self.views as usize,
                            mappings.as_ptr(),
                        )
                    };
                    self.views = 0;
                }
                self.opened(ProtocolObject::from_ref(&*encoder), RENDER);
                self.render = Some(encoder);
                self.pass = MTL4RenderPassDescriptor::new();
                self.primitive = MTLPrimitiveType::Triangle;
            }
            Command::EndRender => {
                let encoder = self.render.take().unwrap();
                self.ended(ProtocolObject::from_ref(&*encoder), RENDER);
            }
            Command::BindRenderPipeline { pipeline } => {
                let p = self.res.renders.get(pipeline);
                self.render().setRenderPipelineState(&p.state);
                self.render().setTriangleFillMode(p.fill);
                self.render().setDepthClipMode(p.clip);
                self.task = p.task;
                self.mesh = p.mesh;
            }
            Command::SetViewport {
                x,
                y,
                width,
                height,
                near,
                far,
            } => self.render().setViewport(MTLViewport {
                originX: x as f64,
                originY: y as f64,
                width: width as f64,
                height: height as f64,
                znear: near as f64,
                zfar: far as f64,
            }),
            Command::SetScissor {
                x,
                y,
                width,
                height,
            } => self.render().setScissorRect(MTLScissorRect {
                x: x as usize,
                y: y as usize,
                width: width as usize,
                height: height as usize,
            }),
            Command::SetCull { cull } => self.render().setCullMode(MTLCullMode(cull as usize)),
            Command::SetFace { face: f } => self.render().setFrontFacingWinding(match f {
                face::CCW => MTLWinding::CounterClockwise,
                face::CW => MTLWinding::Clockwise,
                f => panic!("unknown face {f}"),
            }),
            Command::SetTopology { topology } => {
                self.primitive = MTLPrimitiveType(topology as usize)
            }
            Command::SetDepthState { state } => self
                .render()
                .setDepthStencilState(Some(self.res.depths.get(state))),
            Command::SetStencilReference { front, back } => self
                .render()
                .setStencilFrontReferenceValue_backReferenceValue(front, back),
            Command::SetDepthBias {
                constant,
                slope,
                clamp,
            } => self
                .render()
                .setDepthBias_slopeScale_clamp(constant, slope, clamp),
            Command::SetBlendColor { r, g, b, a } => {
                self.render().setBlendColorRed_green_blue_alpha(r, g, b, a)
            }
            Command::BindVertexBuffer {
                slot,
                buffer,
                offset,
            } => {
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                unsafe { self.table.setAddress_atIndex(address, 16 + slot as usize) };
            }
            Command::BindIndexBuffer {
                buffer,
                offset,
                kind,
            } => {
                let b = self.res.buffers.get(buffer);
                let (kind, size) = match kind {
                    index::U16 => (MTLIndexType::UInt16, 2),
                    index::U32 => (MTLIndexType::UInt32, 4),
                    k => panic!("unknown index {k}"),
                };
                self.index = (
                    b.gpuAddress() + offset,
                    b.length() - offset as usize,
                    kind,
                    size,
                );
            }
            Command::Draw {
                vertices,
                instances,
                first_vertex,
                first_instance,
            } => {
                self.flush();
                unsafe {
                    self.render()
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
                self.flush();
                let (address, length, kind, size) = self.index;
                let skip = first_index as u64 * size;
                unsafe {
                    self.render()
                        .drawIndexedPrimitives_indexCount_indexType_indexBuffer_indexBufferLength_instanceCount_baseVertex_baseInstance(
                            self.primitive,
                            indices as usize,
                            kind,
                            address + skip,
                            length - skip as usize,
                            instances as usize,
                            base_vertex as isize,
                            first_instance as usize,
                        )
                };
            }
            Command::DrawIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.flush();
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                for i in 0..count as u64 {
                    self.render()
                        .drawPrimitives_indirectBuffer(self.primitive, address + i * stride as u64);
                }
            }
            Command::DrawIndexedIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.flush();
                let (index, length, kind, _) = self.index;
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                for i in 0..count as u64 {
                    unsafe {
                        self.render()
                            .drawIndexedPrimitives_indexType_indexBuffer_indexBufferLength_indirectBuffer(
                                self.primitive,
                                kind,
                                index,
                                length,
                                address + i * stride as u64,
                            )
                    };
                }
            }
            Command::DrawMesh { x, y, z } => {
                self.flush();
                self.render()
                    .drawMeshThreadgroups_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                        size(x, y, z),
                        self.task,
                        self.mesh,
                    );
            }
            Command::DrawMeshIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                self.flush();
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                for i in 0..count as u64 {
                    self.render()
                        .drawMeshThreadgroupsWithIndirectBuffer_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                            address + i * stride as u64,
                            self.task,
                            self.mesh,
                        );
                }
            }
            Command::OcclusionHeap { heap } => {
                if let Query::Buffer(buffer) = self.res.queries.get(heap) {
                    unsafe { self.pass.setVisibilityResultBuffer(Some(buffer)) };
                }
            }
            Command::ResetQueries { .. } => {}
            Command::WriteTimestamp { heap, index, stage } => {
                let Query::Timestamp(heap) = self.res.queries.get(heap) else {
                    panic!("write_timestamp needs a timestamp heap");
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
                                render_stage(stage),
                                heap,
                                index,
                            ),
                        _ => self.buffer.writeTimestampIntoHeap_atIndex(heap, index),
                    }
                }
            }
            Command::BeginQuery { index, .. } => unsafe {
                self.render().setVisibilityResultMode_offset(
                    MTLVisibilityResultMode::Counting,
                    index as usize * 8,
                )
            },
            Command::EndQuery { index, .. } => unsafe {
                self.render().setVisibilityResultMode_offset(
                    MTLVisibilityResultMode::Disabled,
                    index as usize * 8,
                )
            },
            Command::ResolveQueries {
                heap,
                first,
                count,
                buffer,
                offset,
            } => {
                let dst = self.res.buffers.get(buffer);
                match self.res.queries.get(heap) {
                    Query::Timestamp(heap) => unsafe {
                        self.buffer
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
                    Query::Buffer(src) => {
                        let encoder = self
                            .buffer
                            .computeCommandEncoder()
                            .ok_or("compute encoder creation failed")?;
                        self.opened(ProtocolObject::from_ref(&*encoder), COMPUTE);
                        unsafe {
                            encoder.copyFromBuffer_sourceOffset_toBuffer_destinationOffset_size(
                                src,
                                first as usize * 8,
                                dst,
                                offset as usize,
                                count as usize * 8,
                            )
                        };
                        self.ended(ProtocolObject::from_ref(&*encoder), COMPUTE);
                    }
                }
            }
            Command::BuildAccel {
                accel,
                scratch,
                scratch_offset,
                update,
            } => {
                let a = self.res.accels.get(accel);
                let range = MTL4BufferRange {
                    bufferAddress: self.res.buffers.get(scratch).gpuAddress() + scratch_offset,
                    length: if update != 0 { a.update } else { a.build },
                };
                unsafe {
                    if update != 0 {
                        self.compute()
                            .refitAccelerationStructure_descriptor_destination_scratchBuffer(
                                &a.raw,
                                &a.desc,
                                Some(&a.raw),
                                range,
                            )
                    } else {
                        self.compute()
                            .buildAccelerationStructure_descriptor_scratchBuffer(
                                &a.raw, &a.desc, range,
                            )
                    }
                }
            }
            Command::TilePass {
                width,
                height,
                imageblock,
                memory,
            } => {
                self.pass.setTileWidth(width as usize);
                self.pass.setTileHeight(height as usize);
                self.pass.setImageblockSampleLength(imageblock as usize);
                self.pass.setThreadgroupMemoryLength(memory as usize);
            }
            Command::DispatchTile { width, height } => {
                self.flush();
                self.render().dispatchThreadsPerTile(size(width, height, 1));
            }
            Command::TileMemory {
                length,
                offset,
                index,
            } => unsafe {
                self.render().setThreadgroupMemoryLength_offset_atIndex(
                    length as usize,
                    offset as usize,
                    index as usize,
                )
            },
            Command::Multiview { views } => self.views = views,
            Command::Callback { index } => {
                self.flush();
                (self
                    .natives
                    .get(index as usize)
                    .expect("callback recorded without its closure"))(&Native {
                    buffer: self.buffer,
                    table: self.table,
                    compute: self.compute.as_deref(),
                    render: self.render.as_deref(),
                })
            }
            Command::ExecuteIndirect {
                indirect,
                start,
                count,
            } => {
                self.flush();
                let icb = self.res.indirects.get(indirect);
                let range = NSRange::new(start as usize, count as usize);
                unsafe {
                    match (&self.compute, &self.render) {
                        (Some(e), _) => e.executeCommandsInBuffer_withRange(icb, range),
                        (_, Some(e)) => e.executeCommandsInBuffer_withRange(icb, range),
                        _ => panic!("execute_indirect outside an encoder"),
                    }
                }
            }
            Command::ResetIndirect {
                indirect,
                start,
                count,
            } => unsafe {
                self.compute().resetCommandsInBuffer_withRange(
                    self.res.indirects.get(indirect),
                    NSRange::new(start as usize, count as usize),
                )
            },
            Command::OptimizeIndirect {
                indirect,
                start,
                count,
            } => unsafe {
                self.compute().optimizeIndirectCommandBuffer_withRange(
                    self.res.indirects.get(indirect),
                    NSRange::new(start as usize, count as usize),
                )
            },
            Command::RateMap { map } => self
                .pass
                .setRasterizationRateMap(Some(self.res.maps.get(map))),
            Command::WriteCompactedSize { accel, heap, index } => {
                let Query::Buffer(buffer) = self.res.queries.get(heap) else {
                    panic!("write_compacted_size needs a compacted heap");
                };
                unsafe {
                    self.compute()
                        .writeCompactedAccelerationStructureSize_toBuffer(
                            &self.res.accels.get(accel).raw,
                            MTL4BufferRange {
                                bufferAddress: buffer.gpuAddress() + index as u64 * 8,
                                length: 8,
                            },
                        )
                }
            }
            Command::CopyAccel { src, dst, compact } => {
                let (s, d) = (&self.res.accels.get(src).raw, &self.res.accels.get(dst).raw);
                unsafe {
                    if compact != 0 {
                        self.compute()
                            .copyAndCompactAccelerationStructure_toAccelerationStructure(s, d)
                    } else {
                        self.compute()
                            .copyAccelerationStructure_toAccelerationStructure(s, d)
                    }
                }
            }
            Command::Upscale {
                upscaler,
                color,
                output,
                depth,
                motion,
                jitter_x,
                jitter_y,
                reset,
            } => {
                let texture = |t: u32| &*self.res.textures.get(t).raw;
                unsafe {
                    match self.res.upscalers.get(upscaler) {
                        Upscaler::Spatial(s) => {
                            s.setColorTexture(Some(texture(color)));
                            s.setOutputTexture(Some(texture(output)));
                            s.setFence(Some(self.fence));
                            s.encodeToCommandBuffer(self.buffer);
                        }
                        Upscaler::Temporal(s) => {
                            s.setColorTexture(Some(texture(color)));
                            s.setDepthTexture(Some(texture(depth)));
                            s.setMotionTexture(Some(texture(motion)));
                            s.setOutputTexture(Some(texture(output)));
                            s.setJitterOffsetX(jitter_x);
                            s.setJitterOffsetY(jitter_y);
                            s.setReset(reset != 0);
                            s.setFence(Some(self.fence));
                            s.encodeToCommandBuffer(self.buffer);
                        }
                    }
                }
                self.waiting = true;
            }
            Command::SetViewports { data } => {
                let mut viewports: Vec<MTLViewport> = data
                    .chunks(24)
                    .map(|c| {
                        let f = |i: usize| {
                            f32::from_le_bytes(c[i * 4..i * 4 + 4].try_into().unwrap()) as f64
                        };
                        MTLViewport {
                            originX: f(0),
                            originY: f(1),
                            width: f(2),
                            height: f(3),
                            znear: f(4),
                            zfar: f(5),
                        }
                    })
                    .collect();
                unsafe {
                    self.render().setViewports_count(
                        NonNull::new(viewports.as_mut_ptr()).unwrap(),
                        viewports.len(),
                    )
                }
            }
            Command::SetScissors { data } => {
                let mut rects: Vec<MTLScissorRect> = data
                    .chunks(16)
                    .map(|c| {
                        let u = |i: usize| {
                            u32::from_le_bytes(c[i * 4..i * 4 + 4].try_into().unwrap()) as usize
                        };
                        MTLScissorRect {
                            x: u(0),
                            y: u(1),
                            width: u(2),
                            height: u(3),
                        }
                    })
                    .collect();
                unsafe {
                    self.render().setScissorRects_count(
                        NonNull::new(rects.as_mut_ptr()).unwrap(),
                        rects.len(),
                    )
                }
            }
            Command::SetDepthBounds { min, max } => {
                if !self.device.supportsFamily(MTLGPUFamily::Apple10) {
                    return Err("depth bounds are not supported");
                }
                self.render().setDepthTestMinBound_maxBound(min, max)
            }
            Command::BeginLabel { text } => {
                let text = NSString::from_str(text);
                match (&self.compute, &self.render) {
                    (Some(e), _) => e.pushDebugGroup(&text),
                    (_, Some(e)) => e.pushDebugGroup(&text),
                    _ => self.buffer.pushDebugGroup(&text),
                }
            }
            Command::EndLabel => match (&self.compute, &self.render) {
                (Some(e), _) => e.popDebugGroup(),
                (_, Some(e)) => e.popDebugGroup(),
                _ => self.buffer.popDebugGroup(),
            },
        }
        Ok(())
    }
}
