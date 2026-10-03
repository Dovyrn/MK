use crate::command;
use crate::mtl::frame::Frame;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::queue::Queue;
use crate::mtl::render::Render;
use crate::mtl::resolve::Resolve;
use crate::mtl::resources::Resources;
use crate::mtl::texture::Texture;
use crate::mtl::{Callback, Native};
use crate::{Command, access, face, format, index, load, stage, store};
use crate::{register, slot};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSRange, NSString};
use objc2_metal::{
    MTL4ArgumentTable, MTL4BufferRange, MTL4CommandBuffer, MTL4CommandEncoder,
    MTL4ComputeCommandEncoder, MTL4RenderCommandEncoder, MTL4RenderPassDescriptor,
    MTL4TimestampGranularity, MTL4VisibilityOptions, MTLBlitOption, MTLBuffer, MTLClearColor,
    MTLCullMode, MTLDevice, MTLIndexType, MTLLoadAction, MTLOrigin, MTLPrimitiveType,
    MTLRenderStages, MTLResidencySet, MTLSamplerState, MTLScissorRect, MTLSize, MTLStages,
    MTLStoreAction, MTLTexture, MTLVertexAmplificationViewMapping, MTLViewport,
    MTLVisibilityResultMode, MTLWinding,
};
use std::ptr::NonNull;

pub struct Recorder<'a> {
    device: &'a ProtocolObject<dyn MTLDevice>,
    residency: &'a ProtocolObject<dyn MTLResidencySet>,
    buffer: &'a ProtocolObject<dyn MTL4CommandBuffer>,
    table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
    resolve: &'a Resolve,
    resolves: Vec<(&'a Texture, &'a Texture, bool)>,
    frame: &'a mut Frame,
    res: &'a Resources,
    natives: &'a [Callback],
    pub compute: Option<Retained<ProtocolObject<dyn MTL4ComputeCommandEncoder>>>,
    pub render: Option<Retained<ProtocolObject<dyn MTL4RenderCommandEncoder>>>,
    pass: Retained<MTL4RenderPassDescriptor>,
    pending: (MTLStages, MTLStages, MTL4VisibilityOptions),
    views: u32,
    compute_pipeline: Option<&'a Pipeline>,
    render_pipeline: Option<&'a Render>,
    state: Vec<&'a [u8]>,
    width: u32,
    height: u32,
    primitive: MTLPrimitiveType,
    index: Option<(u64, usize, MTLIndexType, u64)>,
    push: [u8; 256],
    size: usize,
    dirty: bool,
    stale: bool,
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

pub fn integer(f: u32) -> Option<bool> {
    (format::uint(f) || format::sint(f)).then(|| format::sint(f))
}

pub const VERTICES: usize = (register::STORAGE + slot::STORAGES) as usize;

fn stages(bits: u32) -> MTLStages {
    let same = stage::VERTEX
        | stage::FRAGMENT
        | stage::TILE
        | stage::TASK
        | stage::MESH
        | stage::RESOURCE
        | stage::COMPUTE
        | stage::COPY
        | stage::ACCEL
        | stage::ML;
    let mut s = bits & same;
    if bits & stage::INDIRECT != 0 {
        s |= stage::VERTEX | stage::TASK | stage::MESH | stage::COMPUTE;
    }
    if bits & stage::RAY != 0 {
        s |= stage::COMPUTE;
    }
    if bits & (stage::DEPTH | stage::COLOR) != 0 {
        s |= stage::FRAGMENT;
    }
    MTLStages(s as usize)
}

fn order(
    encoder: &ProtocolObject<dyn MTL4CommandEncoder>,
    mask: MTLStages,
    (from, to, options): (MTLStages, MTLStages, MTL4VisibilityOptions),
) {
    if from.is_empty() || to.is_empty() {
        return;
    }
    if !(to & mask).is_empty() {
        encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(from, to & mask, options);
    }
    encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(from, to, options);
}

fn render_stage(bits: u32) -> MTLRenderStages {
    match bits {
        stage::VERTEX | stage::INDIRECT => MTLRenderStages::Vertex,
        stage::TILE => MTLRenderStages::Tile,
        stage::TASK => MTLRenderStages::Object,
        stage::MESH => MTLRenderStages::Mesh,
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

fn pitch(
    format: u32,
    width: u32,
    length: u32,
    height: u32,
    count: u32,
) -> (usize, usize, MTLBlitOption) {
    let [bytes, w, h] = format::block(format);
    let (bytes, option) = match format {
        format::D24_UNORM_S8 | format::D32_FLOAT_S8 => (4, MTLBlitOption::DepthFromDepthStencil),
        _ => (bytes, MTLBlitOption::None),
    };
    let length = if length == 0 { width } else { length };
    let count = if count == 0 { height } else { count };
    let row = length.div_ceil(w) * bytes;
    (row as usize, (count.div_ceil(h) * row) as usize, option)
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
        resolve: &'a Resolve,
        frame: &'a mut Frame,
        res: &'a Resources,
        natives: &'a [Callback],
    ) -> Self {
        Self {
            device,
            residency: &queue.residency,
            buffer: &queue.buffer,
            table: &queue.table,
            resolve,
            resolves: Vec::new(),
            frame,
            res,
            natives,
            compute: None,
            render: None,
            pass: MTL4RenderPassDescriptor::new(),
            pending: (MTLStages(0), MTLStages(0), MTL4VisibilityOptions::None),
            views: 0,
            compute_pipeline: None,
            render_pipeline: None,
            state: Vec::new(),
            width: 0,
            height: 0,
            primitive: MTLPrimitiveType::Triangle,
            index: None,
            push: [0; 256],
            size: 0,
            dirty: false,
            stale: false,
        }
    }

    fn compute(&self) -> &ProtocolObject<dyn MTL4ComputeCommandEncoder> {
        self.compute.as_deref().expect("checked by Command::enter")
    }

    fn render(&self) -> &ProtocolObject<dyn MTL4RenderCommandEncoder> {
        self.render.as_deref().expect("checked by Command::enter")
    }

    fn opened(&mut self, encoder: &ProtocolObject<dyn MTL4CommandEncoder>, mask: MTLStages) {
        order(encoder, mask, self.pending);
        self.pending = (MTLStages(0), MTLStages(0), MTL4VisibilityOptions::None);
    }

    pub fn release(&mut self) -> Result<(), &'static str> {
        let (from, to, options) = self.pending;
        if from.is_empty() || to.is_empty() {
            return Ok(());
        }
        let encoder = self
            .buffer
            .computeCommandEncoder()
            .ok_or("compute encoder allocation failed")?;
        encoder.barrierAfterQueueStages_beforeStages_visibilityOptions(from, COMPUTE, options);
        encoder.barrierAfterStages_beforeQueueStages_visibilityOptions(from | COMPUTE, to, options);
        encoder.endEncoding();
        self.pending = (MTLStages(0), MTLStages(0), MTL4VisibilityOptions::None);
        Ok(())
    }

    fn resolve(&mut self) -> Result<(), &'static str> {
        if self.resolves.is_empty() {
            return Ok(());
        }
        let encoder = self
            .buffer
            .computeCommandEncoder()
            .ok_or("compute encoder allocation failed")?;
        self.opened(ProtocolObject::from_ref(&*encoder), COMPUTE);
        self.resolve
            .encode(&encoder, std::mem::take(&mut self.resolves));
        encoder.endEncoding();
        Ok(())
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        if self.dirty {
            let address = self
                .frame
                .push(self.device, self.residency, &self.push[..self.size])?;
            unsafe { self.table.setAddress_atIndex(address, 0) };
            self.dirty = false;
        }
        Ok(())
    }

    fn prepare(&mut self) -> Result<(), &'static str> {
        if self.stale && self.render.is_some() {
            self.stale = false;
            for i in 0..self.state.len() {
                let mut bytes = self.state[i];
                self.step(Command::read(&mut bytes)?)?;
            }
        }
        self.flush()
    }

    pub fn run(&mut self, mut bytes: &'a [u8]) -> Result<(), &'static str> {
        while !bytes.is_empty() {
            let at = bytes;
            let command = Command::read(&mut bytes)?;
            if matches!(
                command,
                Command::BindRenderPipeline { .. }
                    | Command::SetCull { .. }
                    | Command::SetFace { .. }
                    | Command::SetTopology { .. }
                    | Command::SetDepthState { .. }
                    | Command::SetStencilReference { .. }
                    | Command::SetDepthBias { .. }
                    | Command::SetBlendColor { .. }
                    | Command::SetDepthBounds { .. }
            ) {
                let kept = &at[..at.len() - bytes.len()];
                self.state.retain(|s| s[..2] != kept[..2]);
                self.state.push(kept);
            }
            self.step(command)?;
        }
        Ok(())
    }

    fn scissor(&self, x: u32, y: u32, width: u32, height: u32) -> MTLScissorRect {
        let x = x.min(self.width);
        let y = y.min(self.height);
        MTLScissorRect {
            x: x as usize,
            y: y as usize,
            width: width.min(self.width - x) as usize,
            height: height.min(self.height - y) as usize,
        }
    }

    fn step(&mut self, command: Command) -> Result<(), &'static str> {
        match command {
            Command::BeginCompute => {
                let encoder = self
                    .buffer
                    .computeCommandEncoder()
                    .ok_or("compute encoder allocation failed")?;
                encoder.setArgumentTable(Some(self.table));
                self.opened(ProtocolObject::from_ref(&*encoder), COMPUTE);
                if let Some(p) = self.compute_pipeline {
                    encoder.setComputePipelineState(&p.state);
                }
                self.compute = Some(encoder);
            }
            Command::EndCompute => {
                let encoder = self.compute.take().unwrap();
                encoder.endEncoding();
            }
            Command::BindPipeline { pipeline } => {
                let p = self.res.pipelines.get(pipeline);
                self.compute().setComputePipelineState(&p.state);
                self.compute_pipeline = Some(p);
            }
            Command::PushConstants { offset, data } => {
                let at = offset as usize;
                self.push[at..at + data.len()].copy_from_slice(data);
                self.size = self.size.max(at + data.len());
                self.dirty = true;
            }
            Command::Dispatch { x, y, z } => {
                let Some(p) = self.compute_pipeline else {
                    return Ok(());
                };
                if [x, y, z].contains(&0) {
                    return Ok(());
                }
                self.flush()?;
                self.compute()
                    .dispatchThreadgroups_threadsPerThreadgroup(size(x, y, z), p.group);
            }
            Command::DispatchIndirect { buffer, offset } => {
                let Some(p) = self.compute_pipeline else {
                    return Ok(());
                };
                self.flush()?;
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                unsafe {
                    self.compute()
                        .dispatchThreadgroupsWithIndirectBuffer_threadsPerThreadgroup(
                            address, p.group,
                        )
                };
            }
            Command::DispatchThreads { x, y, z } => {
                let Some(p) = self.compute_pipeline else {
                    return Ok(());
                };
                if [x, y, z].contains(&0) {
                    return Ok(());
                }
                self.flush()?;
                self.compute()
                    .dispatchThreads_threadsPerThreadgroup(size(x, y, z), p.group);
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
            } => {
                let b = self.res.buffers.get(buffer);
                let size = if size == !0 {
                    b.length() - offset as usize
                } else {
                    size as usize
                };
                unsafe {
                    self.compute().fillBuffer_range_value(
                        b,
                        NSRange::new(offset as usize, size),
                        value,
                    )
                }
            }
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
            }
            | Command::QueueBarrier {
                src_stage,
                src_access,
                dst_stage,
                dst_access,
            } => {
                let host = access::HOST_READ | access::HOST_WRITE;
                let options = if (src_access | dst_access) & host != 0 {
                    MTL4VisibilityOptions::Device
                } else {
                    MTL4VisibilityOptions::None
                };
                let (from, to) = (stages(src_stage), stages(dst_stage));
                let Some(encoder) = &self.compute else {
                    self.pending.0 |= from;
                    self.pending.1 |= to;
                    self.pending.2 |= options;
                    return Ok(());
                };
                if !(from & COMPUTE).is_empty() && !(to & COMPUTE).is_empty() {
                    encoder.barrierAfterEncoderStages_beforeEncoderStages_visibilityOptions(
                        from & COMPUTE,
                        to & COMPUTE,
                        options,
                    );
                }
                order(
                    ProtocolObject::from_ref(&**encoder),
                    COMPUTE,
                    (from, to, options),
                );
            }
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
                let (row, image, option) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute()
                        .copyFromBuffer_sourceOffset_sourceBytesPerRow_sourceBytesPerImage_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin_options(
                            self.res.buffers.get(buffer),
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
                let (row, image, option) = pitch(t.format, width, row_length, height, row_count);
                unsafe {
                    self.compute()
                        .copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage_options(
                            &t.raw,
                            layer as usize,
                            mip as usize,
                            origin(x, y, z),
                            size(width, height, depth),
                            self.res.buffers.get(buffer),
                            offset as usize,
                            row,
                            image,
                            option,
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
                let res = self.res;
                let t = res.textures.get(texture);
                let attachment = unsafe {
                    self.pass
                        .colorAttachments()
                        .objectAtIndexedSubscript(index as usize)
                };
                attachment.setTexture(Some(&t.raw));
                attachment.setLoadAction(load(l));
                let signed = integer(t.format).filter(|_| resolve != NONE);
                attachment.setStoreAction(match signed {
                    Some(_) => MTLStoreAction::Store,
                    None => store(s, resolve != NONE),
                });
                attachment.setClearColor(MTLClearColor {
                    red: r as f64,
                    green: g as f64,
                    blue: b as f64,
                    alpha: a as f64,
                });
                match signed {
                    Some(signed) => self.resolves.push((t, res.textures.get(resolve), signed)),
                    None if resolve != NONE => {
                        attachment.setResolveTexture(Some(&res.textures.get(resolve).raw))
                    }
                    None => {}
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
                self.pass
                    .setRenderTargetArrayLength(layers.max(self.views) as usize);
                let encoder = self
                    .buffer
                    .renderCommandEncoderWithDescriptor(&self.pass)
                    .ok_or("render encoder allocation failed")?;
                let all = MTLRenderStages::Vertex
                    | MTLRenderStages::Fragment
                    | MTLRenderStages::Tile
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
                self.width = width;
                self.height = height;
                self.stale = true;
            }
            Command::EndRender => {
                let encoder = self.render.take().unwrap();
                encoder.endEncoding();
                self.resolve()?;
            }
            Command::BindRenderPipeline { pipeline } => {
                let p = self.res.renders.get(pipeline);
                self.render().setRenderPipelineState(&p.state);
                self.render().setTriangleFillMode(p.fill);
                self.render().setDepthClipMode(p.clip);
                self.primitive = p.primitive;
                self.render_pipeline = Some(p);
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
            } => self
                .render()
                .setScissorRect(self.scissor(x, y, width, height)),
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
            Command::BindUniform {
                slot,
                buffer,
                offset,
                ..
            } => {
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                let index = (register::UNIFORM + slot) as usize;
                unsafe { self.table.setAddress_atIndex(address, index) };
            }
            Command::BindStorage {
                slot,
                buffer,
                offset,
                ..
            } => {
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                let index = (register::STORAGE + slot) as usize;
                unsafe { self.table.setAddress_atIndex(address, index) };
            }
            Command::BindTexture { slot, texture } => {
                let id = self.res.textures.get(texture).raw.gpuResourceID();
                unsafe { self.table.setTexture_atIndex(id, slot as usize) };
            }
            Command::BindImage { slot, texture } => {
                let id = self.res.textures.get(texture).raw.gpuResourceID();
                let index = (register::IMAGE + slot) as usize;
                unsafe { self.table.setTexture_atIndex(id, index) };
            }
            Command::BindSampler { slot, sampler } => {
                let id = self.res.samplers.get(sampler).gpuResourceID();
                unsafe { self.table.setSamplerState_atIndex(id, slot as usize) };
            }
            Command::BindVertexBuffer {
                slot,
                buffer,
                offset,
            } => {
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                unsafe {
                    self.table
                        .setAddress_atIndex(address, VERTICES + slot as usize)
                };
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
                self.index = Some((
                    b.gpuAddress() + offset,
                    b.length().saturating_sub(offset as usize),
                    kind,
                    size,
                ));
            }
            Command::Draw {
                vertices,
                instances,
                first_vertex,
                first_instance,
            } => {
                if self.render_pipeline.is_none() || vertices == 0 || instances == 0 {
                    return Ok(());
                }
                self.prepare()?;
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
                let (Some(_), Some((address, length, kind, size))) =
                    (self.render_pipeline, self.index)
                else {
                    return Ok(());
                };
                if indices == 0 || instances == 0 {
                    return Ok(());
                }
                self.prepare()?;
                let skip = first_index as u64 * size;
                unsafe {
                    self.render()
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
            Command::DrawIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                if self.render_pipeline.is_none() {
                    return Ok(());
                }
                self.prepare()?;
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
                let (Some(_), Some((index, length, kind, _))) = (self.render_pipeline, self.index)
                else {
                    return Ok(());
                };
                self.prepare()?;
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
                let Some(p) = self.render_pipeline else {
                    return Ok(());
                };
                if [x, y, z].contains(&0) {
                    return Ok(());
                }
                self.prepare()?;
                self.render()
                    .drawMeshThreadgroups_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                        size(x, y, z),
                        p.task,
                        p.mesh,
                    );
            }
            Command::DrawMeshIndirect {
                buffer,
                offset,
                count,
                stride,
            } => {
                let Some(p) = self.render_pipeline else {
                    return Ok(());
                };
                self.prepare()?;
                let address = self.res.buffers.get(buffer).gpuAddress() + offset;
                for i in 0..count as u64 {
                    self.render()
                        .drawMeshThreadgroupsWithIndirectBuffer_threadsPerObjectThreadgroup_threadsPerMeshThreadgroup(
                            address + i * stride as u64,
                            p.task,
                            p.mesh,
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
                            .ok_or("compute encoder allocation failed")?;
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
                        encoder.endEncoding();
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
                if self.render_pipeline.is_none() || width == 0 || height == 0 {
                    return Ok(());
                }
                self.prepare()?;
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
                self.flush()?;
                self.release()?;
                self.natives[index as usize](&Native {
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
                if count == 0 {
                    return Ok(());
                }
                self.prepare()?;
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
                    unreachable!()
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
            Command::SetViewports { viewports } => {
                let mut viewports: Vec<MTLViewport> = command::viewports(viewports)
                    .iter()
                    .map(|v| MTLViewport {
                        originX: v.x as f64,
                        originY: v.y as f64,
                        width: v.width as f64,
                        height: v.height as f64,
                        znear: v.near as f64,
                        zfar: v.far as f64,
                    })
                    .collect();
                unsafe {
                    self.render().setViewports_count(
                        NonNull::new(viewports.as_mut_ptr()).unwrap(),
                        viewports.len(),
                    )
                }
            }
            Command::SetScissors { scissors } => {
                let mut rects: Vec<MTLScissorRect> = command::scissors(scissors)
                    .iter()
                    .map(|r| self.scissor(r.x, r.y, r.width, r.height))
                    .collect();
                unsafe {
                    self.render().setScissorRects_count(
                        NonNull::new(rects.as_mut_ptr()).unwrap(),
                        rects.len(),
                    )
                }
            }
            Command::SetDepthBounds { min, max } => {
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
