use super::{Device, id};
use crate::IndirectInfo;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDevice, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLResourceOptions,
};

impl Device {
    pub fn create_indirect(&mut self, info: &IndirectInfo) -> Result<u32, String> {
        let desc = MTLIndirectCommandBufferDescriptor::new();
        desc.setCommandTypes(MTLIndirectCommandType(info.kinds as usize));
        desc.setInheritPipelineState(info.inherit_pipeline);
        desc.setInheritBuffers(info.inherit_buffers);
        desc.setMaxVertexBufferBindCount(info.vertex_buffers as usize);
        desc.setMaxFragmentBufferBindCount(info.fragment_buffers as usize);
        desc.setMaxKernelBufferBindCount(info.kernel_buffers as usize);
        let raw = unsafe {
            self.raw
                .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                    &desc,
                    info.count as usize,
                    MTLResourceOptions::StorageModePrivate
                        | MTLResourceOptions::HazardTrackingModeUntracked,
                )
                .ok_or("indirect command buffer allocation failed")?
        };
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let id = self.res.indirects.insert(raw);
        self.res.table.indirects.set(id, ());
        Ok(id)
    }

    pub fn destroy_indirect(&mut self, indirect: u32) {
        self.res.table.indirects.clear(indirect);
        let raw = self.res.indirects.remove(indirect);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*raw)));
    }

    pub fn indirect_handle(&self, indirect: u32) -> u64 {
        self.res.table.indirects.get(indirect);
        id(self.res.indirects.get(indirect).gpuResourceID())
    }
}
