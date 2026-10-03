use crate::command;
use crate::vk::bindings;
use crate::vk::extensions::Extensions;
use crate::vk::ffi;
use crate::vk::resources::Resources;
use crate::vk::texture::{Texture, count};
use crate::vk::{Callback, Native};
use crate::{Command, DepthInfo, compare, face, format, index, layout, load, stage, store};
use ash::vk;
use std::collections::BTreeMap;
use std::ffi::CString;

const NONE: u32 = !0;

type Slot = (
    vk::DescriptorType,
    vk::DescriptorBufferInfo,
    vk::DescriptorImageInfo,
);

pub struct Recorder<'a> {
    pub device: &'a ash::Device,
    pub ext: &'a Extensions,
    pub buffer: vk::CommandBuffer,
    pub layout: vk::PipelineLayout,
    pub points: &'a [vk::PipelineBindPoint],
    pub res: &'a Resources,
    pub colors: Vec<vk::RenderingAttachmentInfo<'static>>,
    pub depth: Option<(vk::RenderingAttachmentInfo<'static>, bool)>,
    pub stencil: vk::RenderingAttachmentInfo<'static>,
    pub views: u32,
    pub natives: &'a [Callback],
    pub heap: vk::DescriptorSet,
    pub compute: vk::Pipeline,
    pub graphics: vk::Pipeline,
    pub rays: vk::Pipeline,
    pub slots: BTreeMap<u32, Slot>,
    pub push: Vec<u8>,
    pub rate: Option<vk::RenderingFragmentShadingRateAttachmentInfoKHR<'static>>,
    pub defaults: bool,
    pub area: vk::Rect2D,
}

fn load(l: u32) -> vk::AttachmentLoadOp {
    match l {
        load::LOAD => vk::AttachmentLoadOp::LOAD,
        load::CLEAR => vk::AttachmentLoadOp::CLEAR,
        load::DONT_CARE => vk::AttachmentLoadOp::DONT_CARE,
        l => panic!("unknown load {l}"),
    }
}

fn store(s: u32) -> vk::AttachmentStoreOp {
    match s {
        store::STORE => vk::AttachmentStoreOp::STORE,
        store::DONT_CARE => vk::AttachmentStoreOp::DONT_CARE,
        s => panic!("unknown store {s}"),
    }
}

fn compare_op(c: u32) -> vk::CompareOp {
    match c {
        compare::NONE => vk::CompareOp::ALWAYS,
        compare::NEVER..=compare::ALWAYS => vk::CompareOp::from_raw(c as i32 - 1),
        c => panic!("unknown compare {c}"),
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
        (
            stage::FRAGMENT | stage::TILE,
            vk::PipelineStageFlags2::FRAGMENT_SHADER,
        ),
        (stage::TASK, vk::PipelineStageFlags2::TASK_SHADER_EXT),
        (stage::MESH, vk::PipelineStageFlags2::MESH_SHADER_EXT),
        (stage::RAY, vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR),
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
    let foreign = bits & (stage::RESOURCE | stage::ML);
    assert!(foreign == 0, "stage {foreign} has no Vulkan equivalent");
    let mut flags = vk::PipelineStageFlags2::NONE;
    for (bit, f) in map {
        if bits & bit != 0 {
            flags |= f;
        }
    }
    flags
}

pub fn layout(l: u32) -> vk::ImageLayout {
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

fn write(binding: u32, (kind, buffer, image): &Slot) -> vk::WriteDescriptorSet<'_> {
    let write = vk::WriteDescriptorSet::default()
        .dst_binding(binding)
        .descriptor_type(*kind);
    match *kind {
        vk::DescriptorType::UNIFORM_BUFFER | vk::DescriptorType::STORAGE_BUFFER => {
            write.buffer_info(std::slice::from_ref(buffer))
        }
        _ => write.image_info(std::slice::from_ref(image)),
    }
}

impl Recorder<'_> {
    fn push_slots(&self, writes: &[vk::WriteDescriptorSet]) -> Result<(), &'static str> {
        let push = self
            .ext
            .push
            .as_ref()
            .ok_or("VK_KHR_push_descriptor is not enabled")?;
        for point in self.points {
            unsafe { push.cmd_push_descriptor_set(self.buffer, *point, self.layout, 0, writes) };
        }
        Ok(())
    }

    fn slot(
        &mut self,
        kind: vk::DescriptorType,
        slot: u32,
        buffer: vk::DescriptorBufferInfo,
        image: vk::DescriptorImageInfo,
    ) -> Result<(), &'static str> {
        let binding = bindings::binding(kind, slot);
        self.slots.insert(binding, (kind, buffer, image));
        self.push_slots(&[write(binding, &self.slots[&binding])])
    }

    fn buffer_slot(
        &mut self,
        kind: vk::DescriptorType,
        slot: u32,
        buffer: u32,
        offset: u64,
        size: u64,
    ) -> Result<(), &'static str> {
        let info = vk::DescriptorBufferInfo::default()
            .buffer(self.res.buffers.get(buffer).raw)
            .offset(offset)
            .range(size);
        self.slot(kind, slot, info, vk::DescriptorImageInfo::default())
    }

    fn image_slot(
        &mut self,
        kind: vk::DescriptorType,
        slot: u32,
        info: vk::DescriptorImageInfo,
    ) -> Result<(), &'static str> {
        self.slot(kind, slot, vk::DescriptorBufferInfo::default(), info)
    }

    fn bind_heap(&self) {
        for point in self.points {
            unsafe {
                self.device.cmd_bind_descriptor_sets(
                    self.buffer,
                    *point,
                    self.layout,
                    1,
                    &[self.heap],
                    &[],
                )
            };
        }
    }

    fn restore(&self) -> Result<(), &'static str> {
        self.bind_heap();
        let writes: Vec<_> = self.slots.iter().map(|(b, s)| write(*b, s)).collect();
        if !writes.is_empty() {
            self.push_slots(&writes)?;
        }
        let bound = [
            (vk::PipelineBindPoint::COMPUTE, self.compute),
            (vk::PipelineBindPoint::GRAPHICS, self.graphics),
            (vk::PipelineBindPoint::RAY_TRACING_KHR, self.rays),
        ];
        for (point, pipeline) in bound {
            if pipeline != vk::Pipeline::null() {
                unsafe { self.device.cmd_bind_pipeline(self.buffer, point, pipeline) };
            }
        }
        if !self.push.is_empty() {
            unsafe {
                self.device.cmd_push_constants(
                    self.buffer,
                    self.layout,
                    vk::ShaderStageFlags::ALL,
                    0,
                    &self.push,
                )
            };
        }
        Ok(())
    }

    fn stamp(&self, bit: u32) -> vk::PipelineStageFlags2 {
        let graphics = self.points.contains(&vk::PipelineBindPoint::GRAPHICS);
        let mesh = graphics && self.ext.mesh.is_some();
        let (flag, valid) = match bit {
            stage::VERTEX => (vk::PipelineStageFlags2::VERTEX_SHADER, graphics),
            stage::FRAGMENT | stage::TILE => (vk::PipelineStageFlags2::FRAGMENT_SHADER, graphics),
            stage::TASK => (vk::PipelineStageFlags2::TASK_SHADER_EXT, mesh),
            stage::MESH => (vk::PipelineStageFlags2::MESH_SHADER_EXT, mesh),
            stage::RAY => (
                vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                self.ext.rays.is_some(),
            ),
            stage::INDIRECT => (vk::PipelineStageFlags2::DRAW_INDIRECT, true),
            stage::DEPTH => (vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS, graphics),
            stage::COLOR => (vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT, graphics),
            stage::COMPUTE | stage::ML => (vk::PipelineStageFlags2::COMPUTE_SHADER, true),
            stage::COPY | stage::RESOURCE => (vk::PipelineStageFlags2::ALL_TRANSFER, true),
            stage::ACCEL => (
                vk::PipelineStageFlags2::ACCELERATION_STRUCTURE_BUILD_KHR,
                self.ext.accel.is_some(),
            ),
            _ => (vk::PipelineStageFlags2::ALL_COMMANDS, true),
        };
        if valid {
            flag
        } else {
            vk::PipelineStageFlags2::ALL_COMMANDS
        }
    }

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
            .image_view(self.res.textures.get(texture).target)
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

    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), &'static str> {
        self.bind_heap();
        while !bytes.is_empty() {
            self.step(Command::read(&mut bytes)?)?;
        }
        Ok(())
    }

    fn step(&mut self, command: Command) -> Result<(), &'static str> {
        let (d, b) = (self.device, self.buffer);
        unsafe {
            match command {
                Command::BeginCompute | Command::EndCompute => {}
                Command::BindPipeline { pipeline } => {
                    self.compute = *self.res.pipelines.get(pipeline);
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::COMPUTE, self.compute)
                }
                Command::PushConstants { offset, data } => {
                    let range = offset as usize..offset as usize + data.len();
                    if self.push.len() < range.end {
                        self.push.resize(range.end, 0);
                    }
                    self.push[range].copy_from_slice(data);
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
                } => {
                    let barrier = vk::MemoryBarrier2::default()
                        .src_stage_mask(stages(src_stage))
                        .src_access_mask(vk::AccessFlags2::from_raw(src_access as u64))
                        .dst_stage_mask(stages(dst_stage))
                        .dst_access_mask(vk::AccessFlags2::from_raw(dst_access as u64));
                    let barriers = [barrier];
                    let info = vk::DependencyInfo::default().memory_barriers(&barriers);
                    d.cmd_pipeline_barrier2(b, &info)
                }
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
                            base_mip_level: t.base_mip + base_mip,
                            level_count: count(mips, base_mip, t.mips),
                            base_array_layer: t.base_layer + base_layer,
                            layer_count: count(layers, base_layer, t.layers),
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
                    let f = self.res.textures.get(texture).format;
                    let color = [r, g, blue, a];
                    let clear = vk::ClearValue {
                        color: if format::sint(f) {
                            vk::ClearColorValue {
                                int32: color.map(|v| v as i32),
                            }
                        } else if format::uint(f) {
                            vk::ClearColorValue {
                                uint32: color.map(|v| v as u32),
                            }
                        } else {
                            vk::ClearColorValue { float32: color }
                        },
                    };
                    let mut info = self.attachment(texture, l, load, store, clear);
                    if resolve != NONE {
                        let mode = if format::sint(f) || format::uint(f) {
                            vk::ResolveModeFlags::SAMPLE_ZERO
                        } else {
                            vk::ResolveModeFlags::AVERAGE
                        };
                        info = info
                            .resolve_mode(mode)
                            .resolve_image_view(self.res.textures.get(resolve).target)
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
                    d.cmd_set_line_width(b, 1.0);
                    self.area = area;
                    if self.defaults {
                        return Ok(());
                    }
                    self.defaults = true;
                    d.cmd_set_cull_mode(b, vk::CullModeFlags::NONE);
                    d.cmd_set_front_face(b, vk::FrontFace::COUNTER_CLOCKWISE);
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
                    let (raw, topology) = *self.res.renders.get(pipeline);
                    self.graphics = raw;
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::GRAPHICS, raw);
                    d.cmd_set_primitive_topology(b, topology);
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
                        face::CW => vk::FrontFace::CLOCKWISE,
                        f => panic!("unknown face {f}"),
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
                Command::BindUniform {
                    slot,
                    buffer,
                    offset,
                    size,
                } => self.buffer_slot(
                    vk::DescriptorType::UNIFORM_BUFFER,
                    slot,
                    buffer,
                    offset,
                    size,
                )?,
                Command::BindStorage {
                    slot,
                    buffer,
                    offset,
                    size,
                } => self.buffer_slot(
                    vk::DescriptorType::STORAGE_BUFFER,
                    slot,
                    buffer,
                    offset,
                    size,
                )?,
                Command::BindTexture { slot, texture } => {
                    let info = vk::DescriptorImageInfo::default()
                        .image_view(self.res.textures.get(texture).view)
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                    self.image_slot(vk::DescriptorType::SAMPLED_IMAGE, slot, info)?
                }
                Command::BindImage { slot, texture } => {
                    let info = vk::DescriptorImageInfo::default()
                        .image_view(self.res.textures.get(texture).view)
                        .image_layout(vk::ImageLayout::GENERAL);
                    self.image_slot(vk::DescriptorType::STORAGE_IMAGE, slot, info)?
                }
                Command::BindSampler { slot, sampler } => {
                    let info =
                        vk::DescriptorImageInfo::default().sampler(*self.res.samplers.get(sampler));
                    self.image_slot(vk::DescriptorType::SAMPLER, slot, info)?
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
                        index::U32 => vk::IndexType::UINT32,
                        k => panic!("unknown index {k}"),
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
                Command::DrawIndirectCount {
                    buffer,
                    offset,
                    count,
                    count_offset,
                    max,
                    stride,
                } => d.cmd_draw_indirect_count(
                    b,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    self.res.buffers.get(count).raw,
                    count_offset,
                    max,
                    if stride == 0 { 16 } else { stride },
                ),
                Command::DrawIndexedIndirectCount {
                    buffer,
                    offset,
                    count,
                    count_offset,
                    max,
                    stride,
                } => d.cmd_draw_indexed_indirect_count(
                    b,
                    self.res.buffers.get(buffer).raw,
                    offset,
                    self.res.buffers.get(count).raw,
                    count_offset,
                    max,
                    if stride == 0 { 20 } else { stride },
                ),
                Command::DrawMesh { x, y, z } => self
                    .ext
                    .mesh
                    .as_ref()
                    .unwrap()
                    .cmd_draw_mesh_tasks(b, x, y, z),
                Command::DrawMeshIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if count > 1 && self.ext.features.multi_draw_indirect == 0 {
                        return Err("multi draw indirect is not supported");
                    }
                    self.ext
                        .mesh
                        .as_ref()
                        .unwrap()
                        .cmd_draw_mesh_tasks_indirect(
                            b,
                            self.res.buffers.get(buffer).raw,
                            offset,
                            count,
                            stride,
                        )
                }
                Command::OcclusionHeap { .. } => {}
                Command::ResetQueries { heap, first, count } => {
                    d.cmd_reset_query_pool(b, self.res.queries.get(heap).pool, first, count)
                }
                Command::WriteTimestamp { heap, index, stage } => {
                    let pool = self.res.queries.get(heap).pool;
                    d.cmd_write_timestamp2(b, self.stamp(stage), pool, index)
                }
                Command::BeginQuery { heap, index } => {
                    let q = self.res.queries.get(heap);
                    d.cmd_begin_query(b, q.pool, index, q.flags)
                }
                Command::EndQuery { heap, index } => {
                    d.cmd_end_query(b, self.res.queries.get(heap).pool, index)
                }
                Command::ResolveQueries {
                    heap,
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
                        q.stride,
                        vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
                    )
                }
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
                        .accel
                        .as_ref()
                        .unwrap()
                        .cmd_build_acceleration_structures(b, &[info], &[&a.ranges]);
                }
                Command::Multiview { views } => self.views = views,
                Command::Callback { index } => {
                    self.natives[index as usize](&Native {
                        device: d,
                        buffer: b,
                    });
                    self.restore()?
                }
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
                            self.compute
                        } else {
                            self.graphics
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
                    (self.ext.generated.as_ref().unwrap().execute)(b, vk::FALSE, &info);
                }
                Command::BindRayPipeline { pipeline } => {
                    self.rays = self.res.rays.get(pipeline).0;
                    d.cmd_bind_pipeline(b, vk::PipelineBindPoint::RAY_TRACING_KHR, self.rays)
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
                    self.ext.rays.as_ref().unwrap().cmd_trace_rays(
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
                    let rates = self.ext.rates.as_ref().unwrap();
                    (rates.fp().cmd_set_fragment_shading_rate_khr)(b, &size, &ops);
                }
                Command::RateAttachment {
                    texture,
                    layout: l,
                    texel_width,
                    texel_height,
                } => {
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
                Command::WriteCompactedSize { accel, heap, index } => self
                    .ext
                    .accel
                    .as_ref()
                    .unwrap()
                    .cmd_write_acceleration_structures_properties(
                        b,
                        &[self.res.accels.get(accel).raw],
                        vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR,
                        self.res.queries.get(heap).pool,
                        index,
                    ),
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
                        .accel
                        .as_ref()
                        .unwrap()
                        .cmd_copy_acceleration_structure(b, &info)
                }
                Command::SetViewports { viewports } => {
                    let viewports: Vec<_> = command::viewports(viewports)
                        .iter()
                        .map(|v| vk::Viewport {
                            x: v.x,
                            y: v.y,
                            width: v.width,
                            height: v.height,
                            min_depth: v.near,
                            max_depth: v.far,
                        })
                        .collect();
                    d.cmd_set_viewport_with_count(b, &viewports);
                    d.cmd_set_scissor_with_count(b, &vec![self.area; viewports.len()])
                }
                Command::SetScissors { scissors } => {
                    let rects: Vec<_> = command::scissors(scissors)
                        .iter()
                        .map(|r| vk::Rect2D {
                            offset: vk::Offset2D {
                                x: r.x as i32,
                                y: r.y as i32,
                            },
                            extent: vk::Extent2D {
                                width: r.width,
                                height: r.height,
                            },
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
                    let c = self.ext.conditional.as_ref().unwrap();
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
                    let name = CString::new(text).unwrap();
                    let label = vk::DebugUtilsLabelEXT::default().label_name(&name);
                    let debug = self.ext.debug.as_ref().unwrap();
                    debug.cmd_begin_debug_utils_label(b, &label);
                }
                Command::EndLabel => {
                    let debug = self
                        .ext
                        .debug
                        .as_ref()
                        .ok_or("VK_EXT_debug_utils is not enabled")?;
                    debug.cmd_end_debug_utils_label(b);
                }
                Command::SetLineWidth { width } => d.cmd_set_line_width(b, width),
            }
        }
        Ok(())
    }
}
