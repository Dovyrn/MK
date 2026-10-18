use crate::Error;
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
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, Error> {
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(31);
        desc.setMaxTextureBindCount(128);
        desc.setMaxSamplerStateBindCount(16);
        Ok(Self {
            allocator: device
                .newCommandAllocator()
                .ok_or_else(|| Error::OutOfMemory("command allocator allocation failed".into()))?,
            table: device
                .newArgumentTableWithDescriptor_error(&desc)
                .map_err(Error::from)?,
            buffers: RefCell::new(Vec::new()),
            used: Cell::new(0),
        })
    }

    pub fn next(
        &self,
        device: &ProtocolObject<dyn MTLDevice>,
    ) -> Result<Retained<ProtocolObject<dyn MTL4CommandBuffer>>, Error> {
        let mut buffers = self.buffers.borrow_mut();
        let i = self.used.get();
        if i == buffers.len() {
            buffers.push(
                device
                    .newCommandBuffer()
                    .ok_or_else(|| Error::OutOfMemory("command buffer allocation failed".into()))?,
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
