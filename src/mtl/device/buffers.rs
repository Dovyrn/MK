use super::{Device, resource_options};
use crate::Buffer;
use crate::Error;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLResource, MTLStorageMode};

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: crate::Memory) -> Result<Buffer, Error> {
        autoreleasepool(|_| {
            self.caps.check_buffer(size, memory)?;
            let buffer = self
                .raw
                .newBufferWithLength_options(size as usize, resource_options(memory))
                .ok_or_else(|| Error::OutOfMemory("buffer allocation failed".into()))?;
            let mappable = buffer.storageMode() != MTLStorageMode::Private;
            let id = self.res.buffers.insert(buffer);
            self.res.table.add_buffer(id, size, mappable);
            Ok(id)
        })
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> Buffer {
        autoreleasepool(|_| {
            let size = raw.length() as u64;
            let mappable = raw.storageMode() != MTLStorageMode::Private;
            let id = self.res.buffers.insert(raw);
            self.res.table.add_buffer(id, size, mappable);
            self.res.table.residency.exempt_buffer(id);
            id
        })
    }

    pub fn raw_buffer(&self, buffer: Buffer) -> &ProtocolObject<dyn MTLBuffer> {
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer)
    }

    pub fn destroy_buffer(&mut self, buffer: Buffer) {
        self.res.table.drop_buffer(buffer);
        self.res.buffers.remove(buffer);
    }

    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    pub unsafe fn map_buffer<T: Copy>(&self, buffer: Buffer) -> &mut [T] {
        let b = self.res.buffers.get(buffer);
        unsafe { self.res.table.mapping(buffer, b.contents().as_ptr().cast()) }
    }

    pub fn unmap_buffer(&self, buffer: Buffer) {
        self.res.table.buffers.get(buffer);
    }

    pub fn buffer_address(&self, buffer: Buffer) -> u64 {
        self.res.table.buffers.get(buffer);
        self.caps.assert_addresses();
        self.res.buffers.get(buffer).gpuAddress()
    }
}
