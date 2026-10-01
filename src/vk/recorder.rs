use crate::slots::Slots;
use crate::vk::buffer::Buffer;
use crate::vk::texture::Texture;
use crate::{Command, layout, stage};
use ash::ext::debug_utils;
use ash::vk;
use std::ffi::CString;

pub struct Recorder<'a> {
    pub device: &'a ash::Device,
    pub debug: Option<&'a debug_utils::Device>,
    pub buffer: vk::CommandBuffer,
    pub layout: vk::PipelineLayout,
    pub buffers: &'a Slots<Buffer>,
    pub textures: &'a Slots<Texture>,
    pub pipelines: &'a Slots<vk::Pipeline>,
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
        let t = self.textures.get(texture);
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
        for point in [
            vk::PipelineBindPoint::COMPUTE,
            vk::PipelineBindPoint::GRAPHICS,
        ] {
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
            self.step(Command::read(&mut bytes)?);
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

    fn step(&mut self, command: Command) {
        let (d, b) = (self.device, self.buffer);
        unsafe {
            match command {
                Command::BeginCompute | Command::EndCompute => {}
                Command::BindPipeline { pipeline } => d.cmd_bind_pipeline(
                    b,
                    vk::PipelineBindPoint::COMPUTE,
                    *self.pipelines.get(pipeline),
                ),
                Command::PushConstants { offset, data } => {
                    d.cmd_push_constants(b, self.layout, vk::ShaderStageFlags::ALL, offset, data)
                }
                Command::Dispatch { x, y, z } => d.cmd_dispatch(b, x, y, z),
                Command::DispatchIndirect { buffer, offset } => {
                    d.cmd_dispatch_indirect(b, self.buffers.get(buffer).raw, offset)
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
                        self.buffers.get(src).raw,
                        self.buffers.get(dst).raw,
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
                    self.buffers.get(buffer).raw,
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
                    old,
                    new,
                    base_mip,
                    mips,
                    base_layer,
                    layers,
                } => {
                    let t = self.textures.get(texture);
                    let barrier = vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(stages(src_stage))
                        .src_access_mask(vk::AccessFlags2::from_raw(src_access as u64))
                        .dst_stage_mask(stages(dst_stage))
                        .dst_access_mask(vk::AccessFlags2::from_raw(dst_access as u64))
                        .old_layout(layout(old))
                        .new_layout(layout(new))
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
                        self.buffers.get(buffer).raw,
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
                        self.buffers.get(buffer).raw,
                        &[region],
                    )
                }
                Command::BeginLabel { text } => {
                    if let Some(debug) = self.debug {
                        let name = CString::new(text).unwrap();
                        let label = vk::DebugUtilsLabelEXT::default().label_name(&name);
                        debug.cmd_begin_debug_utils_label(b, &label);
                    }
                }
                Command::EndLabel => {
                    if let Some(debug) = self.debug {
                        debug.cmd_end_debug_utils_label(b);
                    }
                }
                Command::SetLineWidth { width } => d.cmd_set_line_width(b, width),
            }
        }
    }
}
