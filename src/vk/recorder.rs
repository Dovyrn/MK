mod textures;

use crate::Error;
use crate::Texture;
use crate::command::Command;
use crate::descriptors::set_of;
use crate::sets;
use crate::vk::device::indirect_stride;
use crate::vk::extensions::Extensions;
use crate::vk::resources::Resources;
use crate::vk::texture::TextureObject;
use crate::{
    CompareOp, DepthStencilStateInfo, FrontFace, IndexType, LoadOp, PipelineStages, StoreOp, slot,
};
use ash::vk;
use std::ffi::CString;

pub struct Recorder<'a> {
    pub device: &'a ash::Device,
    pub ext: &'a Extensions,
    pub buffer: vk::CommandBuffer,
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::PipelineLayout,
    pub points: &'static [vk::PipelineBindPoint],
    pub res: &'a Resources,
    pub colors: Vec<vk::RenderingAttachmentInfo<'static>>,
    pub depth: Option<(vk::RenderingAttachmentInfo<'static>, vk::ImageAspectFlags)>,
    pub stencil: vk::RenderingAttachmentInfo<'static>,
    pub heap: vk::DescriptorSet,
    pub point: vk::PipelineBindPoint,
    pub imported: bool,
    pub family: u32,
}

unsafe impl Send for Recorder<'_> {}

fn vk_load_op(l: crate::LoadOp) -> vk::AttachmentLoadOp {
    match l {
        LoadOp::Load => vk::AttachmentLoadOp::LOAD,
        LoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
        LoadOp::Discard => vk::AttachmentLoadOp::DONT_CARE,
    }
}

fn vk_store_op(s: crate::StoreOp) -> vk::AttachmentStoreOp {
    match s {
        StoreOp::Store => vk::AttachmentStoreOp::STORE,
        StoreOp::Discard => vk::AttachmentStoreOp::DONT_CARE,
    }
}

pub fn vk_compare_op(c: crate::CompareOp) -> vk::CompareOp {
    match c {
        CompareOp::None => vk::CompareOp::ALWAYS,
        c => vk::CompareOp::from_raw(c as i32 - 1),
    }
}

fn vk_stages(bits: PipelineStages) -> vk::PipelineStageFlags2 {
    let map = [
        (
            PipelineStages::VERTEX,
            vk::PipelineStageFlags2::INDEX_INPUT
                | vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT
                | vk::PipelineStageFlags2::VERTEX_SHADER,
        ),
        (
            PipelineStages::FRAGMENT,
            vk::PipelineStageFlags2::FRAGMENT_SHADER,
        ),
        (
            PipelineStages::OBJECT,
            vk::PipelineStageFlags2::TASK_SHADER_EXT,
        ),
        (
            PipelineStages::MESH,
            vk::PipelineStageFlags2::MESH_SHADER_EXT,
        ),
        (
            PipelineStages::INDIRECT,
            vk::PipelineStageFlags2::DRAW_INDIRECT,
        ),
        (
            PipelineStages::DEPTH_STENCIL,
            vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
        ),
        (
            PipelineStages::COLOR_ATTACHMENT,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        ),
        (PipelineStages::HOST, vk::PipelineStageFlags2::HOST),
        (
            PipelineStages::COMPUTE,
            vk::PipelineStageFlags2::COMPUTE_SHADER,
        ),
        (PipelineStages::COPY, vk::PipelineStageFlags2::ALL_TRANSFER),
    ];
    let mut flags = vk::PipelineStageFlags2::NONE;
    for (bit, f) in map {
        if bits.contains(bit) {
            flags |= f;
        }
    }
    flags
}

fn vk_access(bits: PipelineStages, reads: bool) -> vk::AccessFlags2 {
    let shader = PipelineStages::VERTEX
        | PipelineStages::FRAGMENT
        | PipelineStages::OBJECT
        | PipelineStages::MESH
        | PipelineStages::COMPUTE;
    let map = [
        (
            shader,
            vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::UNIFORM_READ,
            vk::AccessFlags2::SHADER_STORAGE_WRITE,
        ),
        (
            PipelineStages::VERTEX,
            vk::AccessFlags2::INDEX_READ | vk::AccessFlags2::VERTEX_ATTRIBUTE_READ,
            vk::AccessFlags2::NONE,
        ),
        (
            PipelineStages::FRAGMENT,
            vk::AccessFlags2::INPUT_ATTACHMENT_READ,
            vk::AccessFlags2::NONE,
        ),
        (
            PipelineStages::INDIRECT,
            vk::AccessFlags2::INDIRECT_COMMAND_READ,
            vk::AccessFlags2::NONE,
        ),
        (
            PipelineStages::DEPTH_STENCIL,
            vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ,
            vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
        ),
        (
            PipelineStages::COLOR_ATTACHMENT,
            vk::AccessFlags2::COLOR_ATTACHMENT_READ,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        ),
        (
            PipelineStages::COPY,
            vk::AccessFlags2::TRANSFER_READ,
            vk::AccessFlags2::TRANSFER_WRITE,
        ),
        (
            PipelineStages::HOST,
            vk::AccessFlags2::HOST_READ,
            vk::AccessFlags2::HOST_WRITE,
        ),
    ];
    let mut flags = vk::AccessFlags2::NONE;
    for (bit, read, write) in map {
        if bits.contains(bit) {
            flags |= write;
            if reads {
                flags |= read;
            }
        }
    }
    flags
}

impl Recorder<'_> {
    pub fn clear_pass(&mut self) {
        self.colors.clear();
        self.depth = None;
    }

    fn bind_heap(&self) {
        for point in self.points {
            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    self.buffer,
                    *point,
                    self.layout,
                    sets::BINDLESS,
                    &[self.heap],
                    &[],
                )
            };
        }
    }

    fn timestamp_stage(&self, bit: PipelineStages) -> vk::PipelineStageFlags2 {
        let graphics = self.points.contains(&vk::PipelineBindPoint::GRAPHICS);
        let mesh = graphics && self.ext.mesh.is_some();
        let (flag, valid) = match bit {
            PipelineStages::VERTEX => (vk::PipelineStageFlags2::VERTEX_SHADER, graphics),
            PipelineStages::FRAGMENT => (vk::PipelineStageFlags2::FRAGMENT_SHADER, graphics),
            PipelineStages::OBJECT => (
                vk::PipelineStageFlags2::TASK_SHADER_EXT,
                graphics && self.ext.task,
            ),
            PipelineStages::MESH => (vk::PipelineStageFlags2::MESH_SHADER_EXT, mesh),
            PipelineStages::INDIRECT => (vk::PipelineStageFlags2::DRAW_INDIRECT, true),
            PipelineStages::DEPTH_STENCIL => {
                (vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS, graphics)
            }
            PipelineStages::COLOR_ATTACHMENT => {
                (vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, graphics)
            }
            PipelineStages::COMPUTE => (vk::PipelineStageFlags2::COMPUTE_SHADER, true),
            PipelineStages::COPY => (vk::PipelineStageFlags2::ALL_TRANSFER, true),
            _ => (vk::PipelineStageFlags2::ALL_COMMANDS, true),
        };
        if valid {
            flag
        } else {
            vk::PipelineStageFlags2::ALL_COMMANDS
        }
    }

    fn record_depth_state(&self, d: &DepthStencilStateInfo) {
        let (dev, b) = (self.device, self.buffer);
        unsafe {
            dev.cmd_set_depth_test_enable(b, d.compare != CompareOp::None || d.write);
            dev.cmd_set_depth_write_enable(b, d.write);
            dev.cmd_set_depth_compare_op(b, vk_compare_op(d.compare));
            dev.cmd_set_stencil_test_enable(b, d.stencil_test);
            if !d.stencil_test {
                return;
            }
            for (mask, f) in [
                (vk::StencilFaceFlags::FRONT, &d.front),
                (vk::StencilFaceFlags::BACK, &d.back),
            ] {
                dev.cmd_set_stencil_op(
                    b,
                    mask,
                    vk::StencilOp::from_raw(f.fail_op as i32),
                    vk::StencilOp::from_raw(f.pass_op as i32),
                    vk::StencilOp::from_raw(f.depth_fail_op as i32),
                    vk_compare_op(f.compare),
                );
                dev.cmd_set_stencil_compare_mask(b, mask, f.read_mask);
                dev.cmd_set_stencil_write_mask(b, mask, f.write_mask);
            }
        }
    }

    fn attachment_info(
        &self,
        texture: Texture,
        load: LoadOp,
        store: StoreOp,
        clear: vk::ClearValue,
    ) -> vk::RenderingAttachmentInfo<'static> {
        vk::RenderingAttachmentInfo::default()
            .image_view(self.res.textures.get(texture).target)
            .image_layout(vk::ImageLayout::GENERAL)
            .load_op(self::vk_load_op(load))
            .store_op(self::vk_store_op(store))
            .clear_value(clear)
    }

    #[allow(clippy::too_many_arguments)]
    fn buffer_image_copy(
        &self,
        texture: Texture,
        mip: u32,
        layer: u32,
        offset: [u32; 3],
        size: [u32; 3],
        buffer_offset: u64,
        row_length: u32,
        row_count: u32,
    ) -> (&TextureObject, vk::BufferImageCopy) {
        let t = self.res.textures.get(texture);
        let aspect = if t.aspect.contains(vk::ImageAspectFlags::DEPTH) {
            vk::ImageAspectFlags::DEPTH
        } else {
            t.aspect
        };
        let region = vk::BufferImageCopy {
            buffer_offset,
            buffer_row_length: row_length,
            buffer_image_height: row_count,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: aspect,
                mip_level: t.base_mip + mip,
                base_array_layer: t.base_layer + layer,
                layer_count: 1,
            },
            image_offset: vk::Offset3D {
                x: offset[0] as i32,
                y: offset[1] as i32,
                z: offset[2] as i32,
            },
            image_extent: vk::Extent3D {
                width: size[0],
                height: size[1],
                depth: size[2],
            },
        };
        (t, region)
    }

    pub fn bind_heap_set(&self) {
        self.bind_heap();
    }

    pub fn raw(&self) -> vk::CommandBuffer {
        self.buffer
    }

    pub fn end(&mut self) -> Result<(), Error> {
        if self.imported {
            return Ok(());
        }
        unsafe { self.device.end_command_buffer(self.buffer) }.map_err(Error::from)
    }

    fn usable(&self, bits: PipelineStages) -> PipelineStages {
        let mut gone = PipelineStages::default();
        if self.ext.mesh.is_none() {
            gone |= PipelineStages::MESH;
        }
        if !self.ext.task {
            gone |= PipelineStages::OBJECT;
        }
        bits & !gone
    }

    fn share(&self, texture: crate::Texture, release: bool, stage: PipelineStages) {
        let t = self.res.textures.get(texture);
        let barrier = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::GENERAL)
            .image(t.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: t.aspect,
                base_mip_level: t.base_mip,
                level_count: t.mips,
                base_array_layer: t.base_layer,
                layer_count: t.layers,
            });
        let barrier = if release {
            barrier
                .src_stage_mask(vk_stages(stage))
                .src_access_mask(vk_access(stage, false))
                .src_queue_family_index(self.family)
                .dst_queue_family_index(vk::QUEUE_FAMILY_EXTERNAL)
        } else {
            barrier
                .dst_stage_mask(vk_stages(stage))
                .dst_access_mask(vk_access(stage, true))
                .src_queue_family_index(vk::QUEUE_FAMILY_EXTERNAL)
                .dst_queue_family_index(self.family)
        };
        let barriers = [barrier];
        let info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
        unsafe { self.device.cmd_pipeline_barrier2(self.buffer, &info) };
    }

    fn barrier(&self, src: PipelineStages, dst: PipelineStages) {
        if src.is_empty() {
            return;
        }
        let (src, dst) = (self.usable(src), self.usable(dst));
        let barrier = vk::MemoryBarrier2::default()
            .src_stage_mask(vk_stages(src))
            .src_access_mask(vk_access(src, false))
            .dst_stage_mask(vk_stages(dst))
            .dst_access_mask(vk_access(dst, true));
        let barriers = [barrier];
        let info = vk::DependencyInfo::default().memory_barriers(&barriers);
        unsafe { self.device.cmd_pipeline_barrier2(self.buffer, &info) };
    }

    fn bind_set(&self, set: crate::DescriptorSet, offsets: &[u32]) {
        let (raw, pool) = *self.res.descriptor_sets.get(set);
        let bindings = &self.res.descriptor_pools.get(pool).bindings;
        unsafe {
            self.device.cmd_bind_descriptor_sets(
                self.buffer,
                self.point,
                self.pipeline,
                set_of(bindings[0].kind),
                &[raw],
                offsets,
            )
        }
    }

    pub fn step(&mut self, command: Command) -> Result<(), Error> {
        let (d, b) = (self.device, self.buffer);
        unsafe {
            match command {
                Command::BeginComputePass {
                    src_stage,
                    dst_stage,
                } => {
                    self.point = vk::PipelineBindPoint::COMPUTE;
                    self.barrier(src_stage, dst_stage);
                }
                Command::EndComputePass => {}
                Command::BindComputePipeline { pipeline } => {
                    let (raw, layout) = *self.res.pipelines.get(pipeline);
                    self.pipeline = layout;
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::COMPUTE, raw)
                }
                Command::Dispatch { x, y, z } => d.cmd_dispatch(b, x, y, z),
                Command::DispatchIndirect { buffer, offset } => {
                    d.cmd_dispatch_indirect(b, self.res.buffers.get(buffer).raw, offset)
                }
                Command::CopyBufferToBuffer {
                    src,
                    src_offset,
                    dst,
                    dst_offset,
                    size,
                } => {
                    let region = vk::BufferCopy {
                        src_offset,
                        dst_offset,
                        size,
                    };
                    d.cmd_copy_buffer(
                        b,
                        self.res.buffers.get(src).raw,
                        self.res.buffers.get(dst).raw,
                        &[region],
                    )
                }
                Command::FillBuffer {
                    buffer,
                    offset,
                    size,
                    value,
                } => d.cmd_fill_buffer(
                    b,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    size,
                    u32::from_ne_bytes([value; 4]),
                ),
                Command::MemoryBarrier {
                    src_stage,
                    dst_stage,
                } => self.barrier(src_stage, dst_stage),
                Command::PresentBarrier { texture, src_stage } => {
                    let src_stage = self.usable(src_stage);
                    let t = self.res.textures.get(texture);
                    let barrier = vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk_stages(src_stage))
                        .src_access_mask(vk_access(src_stage, false))
                        .old_layout(vk::ImageLayout::GENERAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .image(t.image)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: t.aspect,
                            base_mip_level: t.base_mip,
                            level_count: t.mips,
                            base_array_layer: t.base_layer,
                            layer_count: t.layers,
                        });
                    let barriers = [barrier];
                    let info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
                    d.cmd_pipeline_barrier2(b, &info);
                }
                Command::AcquireSharedTexture { texture, dst_stage } => {
                    self.share(texture, false, self.usable(dst_stage))
                }
                Command::ReleaseSharedTexture { texture, src_stage } => {
                    self.share(texture, true, self.usable(src_stage))
                }
                Command::DiscardTextureContents { textures } => {
                    for chunk in textures.chunks(16) {
                        let mut list = [vk::ImageMemoryBarrier2::default(); 16];
                        for (out, x) in list.iter_mut().zip(chunk) {
                            let t = self.res.textures.get(x.texture);
                            let (src, dst) = (self.usable(x.src_stage), self.usable(x.dst_stage));
                            *out = vk::ImageMemoryBarrier2::default()
                                .src_stage_mask(vk_stages(src))
                                .src_access_mask(vk_access(src, false))
                                .dst_stage_mask(vk_stages(dst))
                                .dst_access_mask(vk_access(dst, true))
                                .old_layout(vk::ImageLayout::UNDEFINED)
                                .new_layout(vk::ImageLayout::GENERAL)
                                .image(t.image)
                                .subresource_range(vk::ImageSubresourceRange {
                                    aspect_mask: t.aspect,
                                    base_mip_level: t.base_mip,
                                    level_count: t.mips,
                                    base_array_layer: t.base_layer,
                                    layer_count: t.layers,
                                });
                        }
                        let list = &list[..chunk.len()];
                        let info = vk::DependencyInfo::default().image_memory_barriers(list);
                        d.cmd_pipeline_barrier2(b, &info);
                    }
                }
                Command::CopyBufferToTexture {
                    buffer,
                    buffer_offset,
                    row_texels,
                    slice_rows: row_count,
                    texture,
                    mip,
                    layer,
                    x,
                    y,
                    z,
                    width,
                    height,
                    depth,
                } => {
                    let (t, region) = self.buffer_image_copy(
                        texture,
                        mip,
                        layer,
                        [x, y, z],
                        [width, height, depth],
                        buffer_offset,
                        row_texels,
                        row_count,
                    );
                    d.cmd_copy_buffer_to_image(
                        b,
                        self.res.buffers.get(buffer).raw,
                        t.image,
                        vk::ImageLayout::GENERAL,
                        &[region],
                    )
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
                    buffer_offset,
                    row_texels,
                    slice_rows: row_count,
                } => {
                    let (t, region) = self.buffer_image_copy(
                        texture,
                        mip,
                        layer,
                        [x, y, z],
                        [width, height, depth],
                        buffer_offset,
                        row_texels,
                        row_count,
                    );
                    d.cmd_copy_image_to_buffer(
                        b,
                        t.image,
                        vk::ImageLayout::GENERAL,
                        self.res.buffers.get(buffer).raw,
                        &[region],
                    )
                }
                Command::CopyTextureToTexture { .. } => self.copy_texture_to_texture(command),
                Command::ColorAttachment {
                    index,
                    texture,
                    load,
                    store,
                    clear: color,
                    resolve,
                } => {
                    let f = self.res.textures.get(texture).format;
                    let clear = vk::ClearValue {
                        color: if f.sint() {
                            vk::ClearColorValue {
                                int32: color.map(|v| v as i32),
                            }
                        } else if f.uint() {
                            vk::ClearColorValue {
                                uint32: color.map(|v| v as u32),
                            }
                        } else {
                            vk::ClearColorValue { float32: color }
                        },
                    };
                    let mut info = self.attachment_info(texture, load, store, clear);
                    if let Some(resolve) = resolve {
                        info = info
                            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
                            .resolve_image_view(self.res.textures.get(resolve).target)
                            .resolve_image_layout(vk::ImageLayout::GENERAL);
                    }
                    let i = index as usize;
                    if self.colors.len() <= i {
                        self.colors
                            .resize(i + 1, vk::RenderingAttachmentInfo::default());
                    }
                    self.colors[i] = info;
                }
                Command::DepthAttachment {
                    texture,
                    load,
                    store,
                    clear_depth: depth,
                    stencil_load,
                    stencil_store,
                    clear_stencil: stencil,
                } => {
                    let clear = vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue { depth, stencil },
                    };
                    let info = self.attachment_info(texture, load, store, clear);
                    let aspect = self.res.textures.get(texture).aspect;
                    self.stencil =
                        self.attachment_info(texture, stencil_load, stencil_store, clear);
                    self.depth = Some((info, aspect));
                }
                Command::BeginRenderPass {
                    width,
                    height,
                    layers,
                    src_stage,
                    dst_stage,
                } => {
                    self.point = vk::PipelineBindPoint::GRAPHICS;
                    self.barrier(src_stage, dst_stage);
                    let area = vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: vk::Extent2D { width, height },
                    };
                    let mut info = vk::RenderingInfo::default()
                        .render_area(area)
                        .layer_count(layers)
                        .color_attachments(&self.colors);
                    if let Some((depth, aspect)) = &self.depth {
                        if aspect.contains(vk::ImageAspectFlags::DEPTH) {
                            info = info.depth_attachment(depth);
                        }
                        if aspect.contains(vk::ImageAspectFlags::STENCIL) {
                            info = info.stencil_attachment(&self.stencil);
                        }
                    }
                    d.cmd_begin_rendering(b, &info);
                    self.clear_pass();
                }
                Command::EndRenderPass => d.cmd_end_rendering(b),
                Command::InputAttachmentBarrier => {
                    let written = vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT
                        | vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                        | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS;
                    let barrier = vk::MemoryBarrier2::default()
                        .src_stage_mask(written | vk::PipelineStageFlags2::FRAGMENT_SHADER)
                        .src_access_mask(
                            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
                                | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        )
                        .dst_stage_mask(written | vk::PipelineStageFlags2::FRAGMENT_SHADER)
                        .dst_access_mask(
                            vk::AccessFlags2::INPUT_ATTACHMENT_READ
                                | vk::AccessFlags2::COLOR_ATTACHMENT_READ
                                | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
                                | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                                | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE,
                        );
                    let barriers = [barrier];
                    let info = vk::DependencyInfo::default()
                        .dependency_flags(vk::DependencyFlags::BY_REGION)
                        .memory_barriers(&barriers);
                    d.cmd_pipeline_barrier2(b, &info);
                }
                Command::TileMemory { .. } | Command::DispatchTile { .. } => {
                    unreachable!("caps.tile_shaders is false on Vulkan")
                }
                Command::BindRenderPipeline { pipeline } => {
                    let (raw, layout, stuck) = *self.res.renders.get(pipeline);
                    self.pipeline = layout;
                    if let Some(face) = stuck {
                        d.cmd_set_front_face(b, face);
                    }
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::GRAPHICS, raw);
                }
                Command::SetCullMode { cull } => {
                    d.cmd_set_cull_mode(b, vk::CullModeFlags::from_raw(cull as u32))
                }
                Command::SetFrontFace { face: f } => d.cmd_set_front_face(
                    b,
                    match f {
                        FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
                        FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
                    },
                ),
                Command::SetPrimitiveTopology { topology } => d.cmd_set_primitive_topology(
                    b,
                    vk::PrimitiveTopology::from_raw(topology as i32),
                ),
                Command::SetDepthStencilState { state } => {
                    self.record_depth_state(self.res.depths.get(state))
                }
                Command::SetStencilReference { front, back } => {
                    d.cmd_set_stencil_reference(b, vk::StencilFaceFlags::FRONT, front);
                    d.cmd_set_stencil_reference(b, vk::StencilFaceFlags::BACK, back);
                }
                Command::SetDepthBias {
                    constant,
                    slope,
                    clamp,
                } => {
                    d.cmd_set_depth_bias_enable(b, constant != 0.0 || slope != 0.0);
                    d.cmd_set_depth_bias(b, constant, clamp, slope);
                }
                Command::SetBlendColor { color } => d.cmd_set_blend_constants(b, &color),
                Command::UseResidencySet { .. } => {}
                Command::BindDescriptorSet { set } => self.bind_set(set, &[]),
                Command::BindDescriptorSetDynamic { set, offsets } => self.bind_set(set, offsets),
                Command::BindVertexBuffer {
                    slot,
                    buffer,
                    offset,
                } => d.cmd_bind_vertex_buffers(
                    b,
                    slot,
                    &[self.res.buffers.get(buffer).raw],
                    &[offset],
                ),
                Command::BindIndexBuffer {
                    buffer,
                    offset,
                    kind,
                } => d.cmd_bind_index_buffer(
                    b,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    match kind {
                        IndexType::U16 => vk::IndexType::UINT16,
                        IndexType::U32 => vk::IndexType::UINT32,
                    },
                ),
                Command::Draw {
                    vertices,
                    instances,
                    first_vertex,
                    first_instance,
                } => d.cmd_draw(b, vertices, instances, first_vertex, first_instance),
                Command::DrawIndexed {
                    indices,
                    instances,
                    first_index,
                    base_vertex,
                    first_instance,
                } => d.cmd_draw_indexed(
                    b,
                    indices,
                    instances,
                    first_index,
                    base_vertex,
                    first_instance,
                ),
                Command::DrawIndirect { buffer, offset } => {
                    d.cmd_draw_indirect(b, self.res.buffers.get(buffer).raw, offset, 1, 16)
                }
                Command::DrawIndexedIndirect { buffer, offset } => {
                    d.cmd_draw_indexed_indirect(b, self.res.buffers.get(buffer).raw, offset, 1, 20)
                }
                Command::DrawMeshTasks { x, y, z } => self
                    .ext
                    .mesh
                    .as_ref()
                    .unwrap()
                    .cmd_draw_mesh_tasks(b, x, y, z),
                Command::ExecuteIndirect {
                    indirect,
                    first,
                    count,
                } => {
                    let e = *self.res.table.indirects.get(indirect);
                    let stride = indirect_stride(e.mesh, e.info.indexed);
                    let raw = self.res.buffers.get(*self.res.indirects.get(indirect)).raw;
                    let at = first as u64 * stride as u64;
                    match (e.mesh, e.info.indexed) {
                        (true, _) => self
                            .ext
                            .mesh
                            .as_ref()
                            .unwrap()
                            .cmd_draw_mesh_tasks_indirect(b, raw, at, count, stride),
                        (_, true) => d.cmd_draw_indexed_indirect(b, raw, at, count, stride),
                        _ => d.cmd_draw_indirect(b, raw, at, count, stride),
                    }
                }
                Command::ExecuteIndirectCount {
                    indirect,
                    buffer,
                    offset,
                } => {
                    let e = *self.res.table.indirects.get(indirect);
                    let stride = indirect_stride(e.mesh, e.info.indexed);
                    let raw = self.res.buffers.get(*self.res.indirects.get(indirect)).raw;
                    let range = self.res.buffers.get(buffer).raw;
                    let (at, max) = (offset + 4, e.info.max_draws);
                    match (e.mesh, e.info.indexed) {
                        (true, _) => self
                            .ext
                            .mesh
                            .as_ref()
                            .unwrap()
                            .cmd_draw_mesh_tasks_indirect_count(b, raw, 0, range, at, max, stride),
                        (_, true) => {
                            d.cmd_draw_indexed_indirect_count(b, raw, 0, range, at, max, stride)
                        }
                        _ => d.cmd_draw_indirect_count(b, raw, 0, range, at, max, stride),
                    }
                }
                Command::DrawMeshTasksIndirect { buffer, offset } => self
                    .ext
                    .mesh
                    .as_ref()
                    .unwrap()
                    .cmd_draw_mesh_tasks_indirect(
                        b,
                        self.res.buffers.get(buffer).raw,
                        offset,
                        1,
                        12,
                    ),
                Command::OcclusionPool { .. } => {}
                Command::ResetQueries {
                    pool: heap,
                    first,
                    count,
                } => d.cmd_reset_query_pool(b, self.res.queries.get(heap).pool, first, count),
                Command::WriteTimestamp {
                    pool: heap,
                    index,
                    stage,
                } => {
                    let pool = self.res.queries.get(heap).pool;
                    d.cmd_write_timestamp2(b, self.timestamp_stage(stage), pool, index)
                }
                Command::BeginOcclusionQuery { pool: heap, index } => {
                    let q = self.res.queries.get(heap);
                    d.cmd_begin_query(b, q.pool, index, q.flags)
                }
                Command::EndOcclusionQuery { pool: heap, index } => {
                    d.cmd_end_query(b, self.res.queries.get(heap).pool, index)
                }
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
                    let q = self.res.queries.get(heap);
                    d.cmd_copy_query_pool_results(
                        b,
                        q.pool,
                        first,
                        count,
                        self.res.buffers.get(buffer).raw,
                        offset,
                        8,
                        vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                    )
                }
                Command::SetViewport { viewports } => {
                    let mut list = [vk::Viewport::default(); slot::VIEWPORTS as usize];
                    let views = viewports.iter();
                    for (v, out) in views.zip(&mut list) {
                        *out = vk::Viewport {
                            x: v.x,
                            y: v.y,
                            width: v.width,
                            height: v.height,
                            min_depth: v.near,
                            max_depth: v.far,
                        };
                    }
                    d.cmd_set_viewport_with_count(b, &list[..viewports.len()])
                }
                Command::SetScissor { scissors } => {
                    let mut list = [vk::Rect2D::default(); slot::VIEWPORTS as usize];
                    for (r, out) in scissors.iter().zip(&mut list) {
                        *out = vk::Rect2D {
                            offset: vk::Offset2D {
                                x: r.x as i32,
                                y: r.y as i32,
                            },
                            extent: vk::Extent2D {
                                width: r.width,
                                height: r.height,
                            },
                        };
                    }
                    d.cmd_set_scissor_with_count(b, &list[..scissors.len()])
                }
                Command::BeginDebugLabel { text } => {
                    if let Some(debug) = &self.ext.debug {
                        let name = CString::new(text).unwrap();
                        let label = vk::DebugUtilsLabelEXT::default().label_name(&name);
                        debug.cmd_begin_debug_utils_label(b, &label);
                    }
                }
                Command::EndDebugLabel => {
                    if let Some(debug) = &self.ext.debug {
                        debug.cmd_end_debug_utils_label(b);
                    }
                }
            }
        }
        Ok(())
    }
}

impl crate::CommandBuffer<'_> {
    pub fn raw(&self) -> vk::CommandBuffer {
        self.recording.recorder.raw()
    }
}

impl crate::ComputePass<'_, '_> {
    pub fn raw(&self) -> vk::CommandBuffer {
        self.recording.recorder.raw()
    }
}

impl crate::RenderPass<'_, '_> {
    pub fn raw(&self) -> vk::CommandBuffer {
        self.recording.recorder.raw()
    }
}
