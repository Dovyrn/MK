use super::{Device, id};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDevice, MTLIndirectCommandBuffer, MTLIndirectCommandBufferDescriptor,
    MTLIndirectCommandType, MTLResourceOptions,
};

impl Device {
    pub fn create_indirect_buffer(&mut self, size: u32, pipeline: u32) -> Result<u32, String> {
        let entry = self.res.table.indirect(size, pipeline);
        autoreleasepool(|_| {
            let desc = MTLIndirectCommandBufferDescriptor::new();
            desc.setCommandTypes(MTLIndirectCommandType::Draw);
            desc.setInheritPipelineState(true);
            desc.setInheritBuffers(true);
            let raw = unsafe {
                self.raw
                    .newIndirectCommandBufferWithDescriptor_maxCommandCount_options(
                        &desc,
                        size as usize,
                        MTLResourceOptions::StorageModePrivate
                            | MTLResourceOptions::HazardTrackingModeUntracked,
                    )
                    .ok_or("indirect command buffer allocation failed")?
            };
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let id = self.res.indirects.insert(raw);
            self.res.table.indirects.set(id, entry);
            Ok(id)
        })
    }

    pub fn destroy_indirect_buffer(&mut self, indirect: u32) {
        self.res.table.indirects.clear(indirect);
        let raw = self.res.indirects.remove(indirect);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*raw)));
    }

    pub fn indirect_handle(&self, indirect: u32) -> Result<(u64, u64), String> {
        let (_, topology) = *self.res.table.indirects.get(indirect);
        let handle = id(self.res.indirects.get(indirect).gpuResourceID());
        Ok((handle, topology as u64))
    }
}
