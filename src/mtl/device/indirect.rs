use super::{Device, id};
use crate::Error;
use crate::mtl::indirect::Indirect;
use crate::{IndirectBuffer, IndirectBufferInfo};
use objc2::rc::autoreleasepool;
use objc2_metal::{
    MTLBuffer, MTLDevice, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLResourceOptions,
};

impl Device {
    pub fn create_indirect_buffer(
        &mut self,
        info: &IndirectBufferInfo,
    ) -> Result<IndirectBuffer, Error> {
        let entry = self.res.table.check_indirect(info);
        self.caps.check_indirect(entry.mesh, info.draws)?;
        let render = self.res.renders.get(info.pipeline);
        let (task, mesh) = (render.task, render.mesh);
        let groups = [
            task.width,
            task.height,
            task.depth,
            mesh.width,
            mesh.height,
            mesh.depth,
        ]
        .map(|n| n as u32);
        autoreleasepool(|_| {
            let desc = MTLIndirectCommandBufferDescriptor::new();
            desc.setCommandTypes(match (entry.mesh, info.indexed) {
                (true, _) => MTLIndirectCommandType::DrawMeshThreadgroups,
                (_, true) => MTLIndirectCommandType::DrawIndexed,
                _ => MTLIndirectCommandType::Draw,
            });
            desc.setInheritPipelineState(true);
            desc.setInheritBuffers(true);
            let raw = unsafe {
                self.raw
                    .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                        &desc,
                        info.draws as usize,
                        MTLResourceOptions::StorageModePrivate
                            | MTLResourceOptions::HazardTrackingModeUntracked,
                    )
                    .ok_or_else(|| {
                        Error::OutOfMemory("indirect command buffer allocation failed".into())
                    })?
            };
            let id = self.res.indirects.insert(Indirect { raw, groups });
            self.res.table.indirects.set(id, entry);
            Ok(id)
        })
    }

    pub fn destroy_indirect_buffer(&mut self, indirect: IndirectBuffer) {
        self.res.table.drop_indirect(indirect);
        self.res.indirects.remove(indirect);
    }

    pub fn indirect_shader_handle(&self, indirect: IndirectBuffer) -> crate::IndirectShaderHandle {
        let info = self.res.table.indirects.get(indirect).info;
        let list = self.res.indirects.get(indirect);
        let indices = if info.indexed {
            self.res.buffers.get(info.indices).gpuAddress() + info.offset
        } else {
            0
        };
        let commands = id(list.raw.gpuResourceID());
        self.res
            .table
            .shader_handle(indirect, commands, indices, list.groups)
    }
}
