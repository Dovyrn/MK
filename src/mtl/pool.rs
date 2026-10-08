use crate::mtl::recorder::VERTICES;
use crate::{register, slot};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTL4CommandAllocator, MTL4CommandBuffer,
    MTLDevice,
};
use std::cell::{Cell, RefCell};

pub struct Pool {
    pub allocator: Retained<ProtocolObject<dyn MTL4CommandAllocator>>,
    pub table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
    buffers: RefCell<Vec<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>>,
    used: Cell<usize>,
}

impl Pool {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(VERTICES + slot::VERTICES as usize);
        desc.setMaxTextureBindCount((register::IMAGE + slot::IMAGES) as usize);
        desc.setMaxSamplerStateBindCount(slot::SAMPLERS as usize);
        Ok(Self {
            allocator: device
                .newCommandAllocator()
                .ok_or("command allocator allocation failed")?,
            table: device
                .newArgumentTableWithDescriptor_error(&desc)
                .map_err(|e| e.localizedDescription().to_string())?,
            buffers: RefCell::new(Vec::new()),
            used: Cell::new(0),
        })
    }

    pub fn next(
        &self,
        device: &ProtocolObject<dyn MTLDevice>,
    ) -> Result<Retained<ProtocolObject<dyn MTL4CommandBuffer>>, String> {
        let mut buffers = self.buffers.borrow_mut();
        let i = self.used.get();
        if i == buffers.len() {
            buffers.push(
                device
                    .newCommandBuffer()
                    .ok_or("command buffer allocation failed")?,
            );
        }
        self.used.set(i + 1);
        Ok(buffers[i].clone())
    }

    pub fn reset(&self) {
        self.allocator.reset();
        self.used.set(0);
    }
}
