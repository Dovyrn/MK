use crate::vk::extensions::Extensions;
use crate::vk::ffi;
use crate::vk::resources::Resources;
use crate::vk::texture::Texture;
use crate::vk::{Callback, Native};
use crate::{Command, DepthInfo, compare, face, index, layout, load, stage, store};
use ash::vk;
use std::ffi::CString;

const NONE: u32 = !0;

const DEBUG: &str = "VK_EXT_debug_utils is not enabled";

pub struct Recorder<'a> {
    pub device: &'a ash::Device,
    pub ext: &'a Extensions,
    pub buffer: vk::CommandBuffer,
    pub layout: vk::PipelineLayout,
    pub res: &'a Resources,
    pub colors: Vec<vk::RenderingAttachmentInfo<'static>>,
    pub depth: Option<(vk::RenderingAttachmentInfo<'static>, bool)>,
    pub stencil: vk::RenderingAttachmentInfo<'static>,
    pub views: u32,
    pub natives: &'a [Callback],
    pub bound: (vk::Pipeline, vk::Pipeline),
    pub rate: Option<vk::RenderingFragmentShadingRateAttachmentInfoKHR<'static>>,
}

fn load(l: u32) -> vk::AttachmentLoadOp {
    match l {
        load::LOAD => vk::AttachmentLoadOp::LOAD,
        load::CLEAR => vk::AttachmentLoadOp::CLEAR,
        _ => vk::AttachmentLoadOp::DONT_CARE,
    }
}

fn store(s: u32) -> vk::AttachmentStoreOp {
    match s {
        store::STORE => vk::AttachmentStoreOp::STORE,
        _ => vk::AttachmentStoreOp::DONT_CARE,
    }
}

fn compare_op(c: u32) -> vk::CompareOp {
    if c == compare::NONE {
        vk::CompareOp::ALWAYS
    } else {
        vk::CompareOp::from_raw(c as i32 - 1)
    }
}

fn stages(bits: u32) -> vk::PipelineStageFlags2 {
    let map = [
        (
            stage::VERTEX,
            vk::PipelineStageFlags2::INDEX_INPUT
                | vk::PipelineStageFlags2::VERTEX_ATTRIBUTE_INPUT
                | vk::PipelineStageFlags2::VERTEX_SHADER,
        ),
        (stage::FRAGMENT, vk::PipelineStageFlags2::FRAGMENT_SHADER),
        (stage::TASK, vk::PipelineStageFlags2::TASK_SHADER_EXT),
        (stage::MESH, vk::PipelineStageFlags2::MESH_SHADER_EXT),
        (stage::INDIRECT, vk::PipelineStageFlags2::DRAW_INDIRECT),
        (
            stage::DEPTH,
            vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
        ),
        (
            stage::COLOR,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        ),
        (stage::HOST, vk::PipelineStageFlags2::HOST),
        (stage::COMPUTE, vk::PipelineStageFlags2::COMPUTE_SHADER),
        (stage::COPY, vk::PipelineStageFlags2::ALL_TRANSFER),
        (
            stage::ACCEL,
            vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
        ),
    ];
    let mut flags = vk::PipelineStageFlags2::NONE;
    for (bit, f) in map {
        if bits & bit != 0 {
            flags |= f;
        }
    }
    flags
}

fn layout(l: u32) -> vk::ImageLayout {
    match l {
        layout::UNDEFINED => vk::ImageLayout::UNDEFINED,
        layout::GENERAL => vk::ImageLayout::GENERAL,
        layout::READ => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        layout::COLOR => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        layout::DEPTH => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        layout::DEPTH_READ => vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        layout::COPY_SRC => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        layout::COPY_DST => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        layout::PRESENT => vk::ImageLayout::PRESENT_SRC_KHR,
        l => panic!("unknown layout {l}"),
    }
}

impl Recorder<'_> {
    fn depth_state(&self, d: &DepthInfo) {
        let (dev, b) = (self.device, self.buffer);
        unsafe {
            dev.cmd_set_depth_test_enable(b, d.compare != compare::NONE || d.write);
            dev.cmd_set_depth_write_enable(b, d.write);
            dev.cmd_set_depth_compare_op(b, compare_op(d.compare));
            dev.cmd_set_stencil_test_enable(b, d.stencil);
            for (mask, f) in [
                (vk::StencilFaceFlags::FRONT, &d.front),
                (vk::StencilFaceFlags::BACK, &d.back),
            ] {
                dev.cmd_set_stencil_op(
                    b,
                    mask,
                    vk::StencilOp::from_raw(f.fail as i32),
                    vk::StencilOp::from_raw(f.pass as i32),
                    vk::StencilOp::from_raw(f.depth_fail as i32),
                    compare_op(f.compare),
                );
                dev.cmd_set_stencil_compare_mask(b, mask, f.read);
                dev.cmd_set_stencil_write_mask(b, mask, f.write);
            }
        }
    }

    fn attachment(
        &self,
        texture: u32,
        layout: u32,
        load: u32,
        store: u32,
        clear: vk::ClearValue,
    ) -> vk::RenderingAttachmentInfo<'static> {
        vk::RenderingAttachmentInfo::default()
            .image_view(self.res.textures.get(texture).view)
            .image_layout(self::layout(layout))
            .load_op(self::load(load))
            .store_op(self::store(store))
            .clear_value(clear)
    }

    #[allow(clippy::too_many_arguments)]
    fn copy(
        &self,
        texture: u32,
        mip: u32,
        layer: u32,
        offset: [u32; 3],
        size: [u32; 3],
        buffer_offset: u64,
        row_length: u32,
        row_count: u32,
    ) -> (&Texture, vk::BufferImageCopy) {
        let t = self.res.textures.get(texture);
        let region = vk::BufferImageCopy {
            buffer_offset,
            buffer_row_length: row_length,
            buffer_image_height: row_count,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: t.aspect,
                mip_level: mip,
                base_array_layer: layer,
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

    pub fn run(&mut self, heap: vk::DescriptorSet, mut bytes: &[u8]) -> Result<(), &'static str> {
        let mut points = vec![
            vk::PipelineBindPoint::COMPUTE,
            vk::PipelineBindPoint::GRAPHICS,
        ];
        if self.ext.rays.is_some() {
            points.push(vk::PipelineBindPoint::RAY_TRACING_KHR);
        }
        for point in points {
            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    self.buffer,
                    point,
                    self.layout,
                    0,
                    &[heap],
                    &[],
                )
            };
        }
        while !bytes.is_empty() {
            self.step(Command::read(&mut bytes)?)?;
        }
        Ok(())
    }

    fn barrier(&self, src_stage: u32, src_access: u32, dst_stage: u32, dst_access: u32) {
        let barrier = vk::MemoryBarrier2::default()
            .src_stage_mask(stages(src_stage))
            .src_access_mask(vk::AccessFlags2::from_raw(src_access as u64))
            .dst_stage_mask(stages(dst_stage))
            .dst_access_mask(vk::AccessFlags2::from_raw(dst_access as u64));
        let barriers = [barrier];
        let info = vk::DependencyInfo::default().memory_barriers(&barriers);
        unsafe { self.device.cmd_pipeline_barrier2(self.buffer, &info) };
    }

    fn step(&mut self, command: Command) -> Result<(), &'static str> {
        let (d, b) = (self.device, self.buffer);
        unsafe {
            match command {
                Command::BeginCompute | Command::EndCompute => {}
                Command::BindPipeline { pipeline } => {
                    self.bound.0 = *self.res.pipelines.get(pipeline);
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::COMPUTE, self.bound.0)
                }
                Command::PushConstants { offset, data } => {
                    d.cmd_push_constants(b, self.layout, vk::ShaderStageFlags::ALL, offset, data)
                }
                Command::Dispatch { x, y, z } => d.cmd_dispatch(b, x, y, z),
                Command::DispatchIndirect { buffer, offset } => {
                    d.cmd_dispatch_indirect(b, self.res.buffers.get(buffer).raw, offset)
                }
                Command::CopyBuffer {
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
                    let t = self.res.textures.get(texture);
                    let barrier = vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(stages(src_stage))
                        .src_access_mask(vk::AccessFlags2::from_raw(src_access as u64))
                        .dst_stage_mask(stages(dst_stage))
                        .dst_access_mask(vk::AccessFlags2::from_raw(dst_access as u64))
                        .old_layout(layout(before))
                        .new_layout(layout(after))
                        .image(t.image)
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: t.aspect,
                            base_mip_level: base_mip,
                            level_count: mips,
                            base_array_layer: base_layer,
                            layer_count: layers,
                        });
                    let barriers = [barrier];
                    let info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
                    d.cmd_pipeline_barrier2(b, &info);
                }
                Command::CopyBufferToTexture {
                    buffer,
                    offset,
                    row_length,
                    row_count,
                    texture,
                    layout: l,
                    mip,
                    layer,
                    x,
                    y,
                    z,
                    width,
                    height,
                    depth,
                } => {
                    let (t, region) = self.copy(
                        texture,
                        mip,
                        layer,
                        [x, y, z],
                        [width, height, depth],
                        offset,
                        row_length,
                        row_count,
                    );
                    d.cmd_copy_buffer_to_image(
                        b,
                        self.res.buffers.get(buffer).raw,
                        t.image,
                        layout(l),
                        &[region],
                    )
                }
                Command::CopyTextureToBuffer {
                    texture,
                    layout: l,
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
                } => {
                    let (t, region) = self.copy(
                        texture,
                        mip,
                        layer,
                        [x, y, z],
                        [width, height, depth],
                        offset,
                        row_length,
                        row_count,
                    );
                    d.cmd_copy_image_to_buffer(
                        b,
                        t.image,
                        layout(l),
                        self.res.buffers.get(buffer).raw,
                        &[region],
                    )
                }
                Command::ColorAttachment {
                    index,
                    texture,
                    layout: l,
                    load,
                    store,
                    r,
                    g,
                    b: blue,
                    a,
                    resolve,
                    resolve_layout,
                } => {
                    let clear = vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [r, g, blue, a],
                        },
                    };
                    let mut info = self.attachment(texture, l, load, store, clear);
                    if resolve != NONE {
                        info = info
                            .resolve_mode(vk::ResolveModeFlags::AVERAGE)
                            .resolve_image_view(self.res.textures.get(resolve).view)
                            .resolve_image_layout(layout(resolve_layout));
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
                    layout: l,
                    load,
                    store,
                    depth,
                    stencil_load,
                    stencil_store,
                    stencil,
                } => {
                    let clear = vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue { depth, stencil },
                    };
                    let info = self.attachment(texture, l, load, store, clear);
                    let has = self
                        .res
                        .textures
                        .get(texture)
                        .aspect
                        .contains(vk::ImageAspectFlags::STENCIL);
                    self.stencil = self.attachment(texture, l, stencil_load, stencil_store, clear);
                    self.depth = Some((info, has));
                }
                Command::BeginRender {
                    width,
                    height,
                    layers,
                } => {
                    let area = vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: vk::Extent2D { width, height },
                    };
                    let mut info = vk::RenderingInfo::default()
                        .render_area(area)
                        .layer_count(layers)
                        .view_mask(if self.views > 1 {
                            (1 << self.views) - 1
                        } else {
                            0
                        })
                        .color_attachments(&self.colors);
                    let mut rate = self.rate.take();
                    if let Some(rate) = &mut rate {
                        info = info.push_next(rate);
                    }
                    if let Some((depth, has)) = &self.depth {
                        info = info.depth_attachment(depth);
                        if *has {
                            info = info.stencil_attachment(&self.stencil);
                        }
                    }
                    d.cmd_begin_rendering(b, &info);
                    self.colors.clear();
                    self.depth = None;
                    self.views = 0;
                    let viewport = vk::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: width as f32,
                        height: height as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    };
                    d.cmd_set_viewport_with_count(b, &[viewport]);
                    d.cmd_set_scissor_with_count(b, &[area]);
                    d.cmd_set_cull_mode(b, vk::CullModeFlags::NONE);
                    d.cmd_set_front_face(b, vk::FrontFace::COUNTER_CLOCKWISE);
                    d.cmd_set_primitive_topology(b, vk::PrimitiveTopology::TRIANGLE_LIST);
                    d.cmd_set_depth_bias_enable(b, false);
                    d.cmd_set_depth_bias(b, 0.0, 0.0, 0.0);
                    d.cmd_set_blend_constants(b, &[0.0; 4]);
                    d.cmd_set_stencil_reference(b, vk::StencilFaceFlags::FRONT_AND_BACK, 0);
                    self.depth_state(&DepthInfo::default());
                    d.cmd_set_depth_bounds_test_enable(b, false);
                    d.cmd_set_depth_bounds(b, 0.0, 1.0);
                    if let Some(rates) = &self.ext.rates {
                        let size = vk::Extent2D {
                            width: 1,
                            height: 1,
                        };
                        let keep = [vk::FragmentShadingRateCombinerOpKHR::KEEP; 2];
                        (rates.fp().cmd_set_fragment_shading_rate_khr)(b, &size, &keep);
                    }
                }
                Command::EndRender => d.cmd_end_rendering(b),
                Command::BindRenderPipeline { pipeline } => {
                    self.bound.1 = *self.res.renders.get(pipeline);
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::GRAPHICS, self.bound.1)
                }
                Command::SetViewport {
                    x,
                    y,
                    width,
                    height,
                    near,
                    far,
                } => d.cmd_set_viewport_with_count(
                    b,
                    &[vk::Viewport {
                        x,
                        y,
                        width,
                        height,
                        min_depth: near,
                        max_depth: far,
                    }],
                ),
                Command::SetScissor {
                    x,
                    y,
                    width,
                    height,
                } => d.cmd_set_scissor_with_count(
                    b,
                    &[vk::Rect2D {
                        offset: vk::Offset2D {
                            x: x as i32,
                            y: y as i32,
                        },
                        extent: vk::Extent2D { width, height },
                    }],
                ),
                Command::SetCull { cull } => {
                    d.cmd_set_cull_mode(b, vk::CullModeFlags::from_raw(cull))
                }
                Command::SetFace { face: f } => d.cmd_set_front_face(
                    b,
                    match f {
                        face::CCW => vk::FrontFace::COUNTER_CLOCKWISE,
                        _ => vk::FrontFace::CLOCKWISE,
                    },
                ),
                Command::SetTopology { topology } => d.cmd_set_primitive_topology(
                    b,
                    vk::PrimitiveTopology::from_raw(topology as i32),
                ),
                Command::SetDepthState { state } => self.depth_state(self.res.depths.get(state)),
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
                Command::SetBlendColor { r, g, b: blue, a } => {
                    d.cmd_set_blend_constants(b, &[r, g, blue, a])
                }
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
                        index::U16 => vk::IndexType::UINT16,
                        _ => vk::IndexType::UINT32,
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
                Command::DrawIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    d.cmd_draw_indirect(b, self.res.buffers.get(buffer).raw, offset, count, stride)
                }
                Command::DrawIndexedIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => d.cmd_draw_indexed_indirect(
                    b,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    count,
                    stride,
                ),
                Command::DrawMesh { x, y, z } => self
                    .ext
                    .mesh
                    .as_ref()
                    .ok_or("VK_EXT_mesh_shader is not enabled")?
                    .cmd_draw_mesh_tasks(b, x, y, z),
                Command::DrawMeshIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => self
                    .ext
                    .mesh
                    .as_ref()
                    .ok_or("VK_EXT_mesh_shader is not enabled")?
                    .cmd_draw_mesh_tasks_indirect(
                        b,
                        self.res.buffers.get(buffer).raw,
                        offset,
                        count,
                        stride,
                    ),
                Command::OcclusionHeap { .. } => {}
                Command::ResetQueries { heap, first, count } => {
                    d.cmd_reset_query_pool(b, self.res.queries.get(heap).0, first, count)
                }
                Command::WriteTimestamp { heap, index, stage } => {
                    d.cmd_write_timestamp2(b, stages(stage), self.res.queries.get(heap).0, index)
                }
                Command::BeginQuery { heap, index } => {
                    let (pool, flags, _) = *self.res.queries.get(heap);
                    d.cmd_begin_query(b, pool, index, flags)
                }
                Command::EndQuery { heap, index } => {
                    d.cmd_end_query(b, self.res.queries.get(heap).0, index)
                }
                Command::ResolveQueries {
                    heap,
                    first,
                    count,
                    buffer,
                    offset,
                } => d.cmd_copy_query_pool_results(
                    b,
                    self.res.queries.get(heap).0,
                    first,
                    count,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    self.res.queries.get(heap).2,
                    vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                ),
                Command::BuildAccel {
                    accel,
                    scratch,
                    scratch_offset,
                    update,
                } => {
                    let a = self.res.accels.get(accel);
                    let mut info = vk::AccelerationStructureBuildGeometryInfoKHR::default()
                        .ty(a.kind)
                        .flags(a.flags)
                        .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                        .dst_acceleration_structure(a.raw)
                        .geometries(&a.geometry)
                        .scratch_data(vk::DeviceOrHostAddressKHR {
                            device_address: self.res.buffers.get(scratch).address + scratch_offset,
                        });
                    if update != 0 {
                        info = info
                            .mode(vk::BuildAccelerationStructureModeKHR::UPDATE)
                            .src_acceleration_structure(a.raw);
                    }
                    self.ext
                        .ray
                        .as_ref()
                        .ok_or("VK_KHR_acceleration_structure is not enabled")?
                        .cmd_build_acceleration_structures(b, &[info], &[&a.ranges]);
                }
                Command::Multiview { views } => self.views = views,
                Command::Callback { index } => (self
                    .natives
                    .get(index as usize)
                    .expect("callback recorded without its closure"))(
                    &Native {
                        device: d,
                        buffer: b,
                    },
                ),
                Command::ExecuteGenerated {
                    layout,
                    indirect,
                    offset,
                    size,
                    preprocess,
                    preprocess_offset,
                    preprocess_size,
                    sequences,
                    count,
                    count_offset,
                    draws,
                } => {
                    let (handle, stages) = *self.res.layouts.get(layout);
                    let mut bound = ffi::PipelineInfo {
                        s_type: ffi::PIPELINE,
                        next: std::ptr::null_mut(),
                        pipeline: if stages == vk::ShaderStageFlags::COMPUTE {
                            self.bound.0
                        } else {
                            self.bound.1
                        },
                    };
                    let address = |buffer: u32, offset: u64| {
                        if buffer == NONE {
                            0
                        } else {
                            self.res.buffers.get(buffer).address + offset
                        }
                    };
                    let info = ffi::GeneratedInfo {
                        s_type: ffi::GENERATED,
                        next: (&raw mut bound).cast(),
                        stages,
                        set: 0,
                        layout: handle,
                        indirect: address(indirect, offset),
                        indirect_size: size,
                        preprocess: address(preprocess, preprocess_offset),
                        preprocess_size,
                        sequences,
                        count: address(count, count_offset),
                        draws,
                    };
                    let generated = self
                        .ext
                        .generated
                        .as_ref()
                        .ok_or("VK_EXT_device_generated_commands is not enabled")?;
                    (generated.execute)(b, vk::FALSE, &info);
                }
                Command::BindRayPipeline { pipeline } => {
                    self.ext
                        .rays
                        .as_ref()
                        .ok_or("VK_KHR_ray_tracing_pipeline is not enabled")?;
                    d.cmd_bind_pipeline(
                        b,
                        vk::PipelineBindPoint::RAY_TRACING_KHR,
                        self.res.rays.get(pipeline).0,
                    )
                }
                Command::TraceRays {
                    raygen,
                    raygen_offset,
                    raygen_stride,
                    raygen_size,
                    miss,
                    miss_offset,
                    miss_stride,
                    miss_size,
                    hit,
                    hit_offset,
                    hit_stride,
                    hit_size,
                    callable,
                    callable_offset,
                    callable_stride,
                    callable_size,
                    width,
                    height,
                    depth,
                } => {
                    let region = |buffer: u32, offset: u64, stride: u64, size: u64| {
                        vk::StridedDeviceAddressRegionKHR {
                            device_address: if buffer == NONE {
                                0
                            } else {
                                self.res.buffers.get(buffer).address + offset
                            },
                            stride,
                            size,
                        }
                    };
                    let rays = self
                        .ext
                        .rays
                        .as_ref()
                        .ok_or("VK_KHR_ray_tracing_pipeline is not enabled")?;
                    rays.cmd_trace_rays(
                        b,
                        &region(raygen, raygen_offset, raygen_stride, raygen_size),
                        &region(miss, miss_offset, miss_stride, miss_size),
                        &region(hit, hit_offset, hit_stride, hit_size),
                        &region(callable, callable_offset, callable_stride, callable_size),
                        width,
                        height,
                        depth,
                    )
                }
                Command::SetShadingRate {
                    width,
                    height,
                    primitive,
                    attachment,
                } => {
                    let size = vk::Extent2D { width, height };
                    let ops = [
                        vk::FragmentShadingRateCombinerOpKHR::from_raw(primitive as i32),
                        vk::FragmentShadingRateCombinerOpKHR::from_raw(attachment as i32),
                    ];
                    let rates = self
                        .ext
                        .rates
                        .as_ref()
                        .ok_or("VK_KHR_fragment_shading_rate is not enabled")?;
                    (rates.fp().cmd_set_fragment_shading_rate_khr)(b, &size, &ops);
                }
                Command::RateAttachment {
                    texture,
                    layout: l,
                    texel_width,
                    texel_height,
                } => {
                    self.ext
                        .rates
                        .as_ref()
                        .ok_or("VK_KHR_fragment_shading_rate is not enabled")?;
                    self.rate = Some(
                        vk::RenderingFragmentShadingRateAttachmentInfoKHR::default()
                            .image_view(self.res.textures.get(texture).view)
                            .image_layout(layout(l))
                            .shading_rate_attachment_texel_size(vk::Extent2D {
                                width: texel_width,
                                height: texel_height,
                            }),
                    )
                }
                Command::WriteCompactedSize { accel, heap, index } => {
                    let ray = self
                        .ext
                        .ray
                        .as_ref()
                        .ok_or("VK_KHR_acceleration_structure is not enabled")?;
                    ray.cmd_write_acceleration_structures_properties(
                        b,
                        &[self.res.accels.get(accel).raw],
                        vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR,
                        self.res.queries.get(heap).0,
                        index,
                    )
                }
                Command::CopyAccel { src, dst, compact } => {
                    let info = vk::CopyAccelerationStructureInfoKHR::default()
                        .src(self.res.accels.get(src).raw)
                        .dst(self.res.accels.get(dst).raw)
                        .mode(if compact != 0 {
                            vk::CopyAccelerationStructureModeKHR::COMPACT
                        } else {
                            vk::CopyAccelerationStructureModeKHR::CLONE
                        });
                    self.ext
                        .ray
                        .as_ref()
                        .ok_or("VK_KHR_acceleration_structure is not enabled")?
                        .cmd_copy_acceleration_structure(b, &info)
                }
                Command::SetViewports { data } => {
                    let viewports: Vec<_> = data
                        .chunks(24)
                        .map(|c| {
                            let f = |i: usize| {
                                f32::from_le_bytes(c[i * 4..i * 4 + 4].try_into().unwrap())
                            };
                            vk::Viewport {
                                x: f(0),
                                y: f(1),
                                width: f(2),
                                height: f(3),
                                min_depth: f(4),
                                max_depth: f(5),
                            }
                        })
                        .collect();
                    d.cmd_set_viewport_with_count(b, &viewports)
                }
                Command::SetScissors { data } => {
                    let rects: Vec<_> = data
                        .chunks(16)
                        .map(|c| {
                            let u = |i: usize| {
                                u32::from_le_bytes(c[i * 4..i * 4 + 4].try_into().unwrap())
                            };
                            vk::Rect2D {
                                offset: vk::Offset2D {
                                    x: u(0) as i32,
                                    y: u(1) as i32,
                                },
                                extent: vk::Extent2D {
                                    width: u(2),
                                    height: u(3),
                                },
                            }
                        })
                        .collect();
                    d.cmd_set_scissor_with_count(b, &rects)
                }
                Command::SetDepthBounds { min, max } => {
                    d.cmd_set_depth_bounds_test_enable(b, min > 0.0 || max < 1.0);
                    d.cmd_set_depth_bounds(b, min, max);
                }
                Command::BeginConditional {
                    buffer,
                    offset,
                    inverted,
                } => {
                    let info = vk::ConditionalRenderingBeginInfoEXT::default()
                        .buffer(self.res.buffers.get(buffer).raw)
                        .offset(offset)
                        .flags(if inverted != 0 {
                            vk::ConditionalRenderingFlagsEXT::INVERTED
                        } else {
                            vk::ConditionalRenderingFlagsEXT::empty()
                        });
                    let c = self
                        .ext
                        .conditional
                        .as_ref()
                        .ok_or("VK_EXT_conditional_rendering is not enabled")?;
                    (c.fp().cmd_begin_conditional_rendering_ext)(b, &info)
                }
                Command::EndConditional => {
                    let c = self
                        .ext
                        .conditional
                        .as_ref()
                        .ok_or("VK_EXT_conditional_rendering is not enabled")?;
                    (c.fp().cmd_end_conditional_rendering_ext)(b)
                }
                Command::BeginLabel { text } => {
                    let debug = self.ext.debug.as_ref().ok_or(DEBUG)?;
                    let name = CString::new(text).expect("label contains a nul byte");
                    let label = vk::DebugUtilsLabelEXT::default().label_name(&name);
                    debug.cmd_begin_debug_utils_label(b, &label);
                }
                Command::EndLabel => {
                    let debug = self.ext.debug.as_ref().ok_or(DEBUG)?;
                    debug.cmd_end_debug_utils_label(b);
                }
                Command::SetLineWidth { width } => d.cmd_set_line_width(b, width),
            }
        }
        Ok(())
    }
}
