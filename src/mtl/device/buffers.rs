use super::{Device, resource_options};
use crate::Buffer;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLResource, MTLStorageMode};

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<Buffer, String> {
        autoreleasepool(|_| {
            self.caps.check_buffer(size, memory)?;
            let buffer = self
                .raw
                .newBufferWithLength_options(size as usize, resource_options(memory))
                .ok_or("buffer allocation failed")?;
            let id = self.res.buffers.insert(buffer);
            self.res.table.buffers.set(id, size);
            Ok(id)
        })
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> Buffer {
        autoreleasepool(|_| {
            let size = raw.length() as u64;
            let id = self.res.buffers.insert(raw);
            self.res.table.buffers.set(id, size);
            self.res.imports.insert(id);
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
        self.res.imports.remove(&buffer);
        self.res.buffers.remove(buffer);
    }

    pub fn map_buffer(&self, buffer: Buffer) -> *mut u8 {
        self.res.table.buffers.get(buffer);
        assert!(
            !self.res.imports.contains(&buffer),
            "map of an imported buffer"
        );
        let b = self.res.buffers.get(buffer);
        assert_ne!(
            b.storageMode(),
            MTLStorageMode::Private,
            "map of a device buffer"
        );
        b.contents().as_ptr().cast()
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
