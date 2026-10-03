use super::{Device, id};
use crate::IndirectInfo;
use crate::mtl::indirect::Indirect;
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBuffer, MTLDevice, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLResourceOptions,
};

impl Device {
    pub fn create_indirect_buffer(&mut self, info: &IndirectInfo) -> Result<u32, String> {
        let entry = self.res.table.indirect(info);
        let render = self.res.renders.get(info.pipeline);
        let (task, mesh) = (render.task, render.mesh);
        let groups = [
            task.width,
            task.height,
            task.depth,
            mesh.width,
            mesh.height,
            mesh.depth,
        ];
        autoreleasepool(|_| {
            let desc = MTLIndirectCommandBufferDescriptor::new();
            desc.setCommandTypes(match (entry.2, info.indexed) {
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
                        info.size as usize,
                        MTLResourceOptions::StorageModePrivate
                            | MTLResourceOptions::HazardTrackingModeUntracked,
                    )
                    .ok_or("indirect command buffer allocation failed")?
            };
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let groups = groups.map(|n| n as u32);
            let id = self.res.indirects.insert(Indirect { raw, groups });
            self.res.table.indirects.set(id, entry);
            Ok(id)
        })
    }

    pub fn destroy_indirect_buffer(&mut self, indirect: u32) {
        self.res.table.indirects.clear(indirect);
        let raw = self.res.indirects.remove(indirect).raw;
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*raw)));
    }

    pub fn indirect_handle(&self, indirect: u32) -> Result<[u32; 8], String> {
        let (info, _, _) = *self.res.table.indirects.get(indirect);
        let list = self.res.indirects.get(indirect);
        let indices = if info.indexed {
            self.res.buffers.get(info.indices).gpuAddress() + info.offset
        } else {
            0
        };
        Ok(self
            .res
            .table
            .handle(indirect, id(list.raw.gpuResourceID()), indices, list.groups))
    }
}
