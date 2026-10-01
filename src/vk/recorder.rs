use crate::slots::Slots;
use crate::vk::buffer::Buffer;
use crate::{Command, stage};
use ash::ext::debug_utils;
use ash::vk;
use std::ffi::CString;

pub struct Recorder<'a> {
    pub device: &'a ash::Device,
    pub debug: Option<&'a debug_utils::Device>,
    pub buffer: vk::CommandBuffer,
    pub layout: vk::PipelineLayout,
    pub buffers: &'a Slots<Buffer>,
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

impl Recorder<'_> {
    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), &'static str> {
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
