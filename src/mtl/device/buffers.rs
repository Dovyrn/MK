use super::{Device, options};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLResource, MTLStorageMode};

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.caps.buffer(size, memory)?;
            let buffer = self
                .raw
                .newBufferWithLength_options(size as usize, options(memory))
                .ok_or("buffer allocation failed")?;
            self.residency.add(ProtocolObject::from_ref(&*buffer));
            let id = self.res.buffers.insert(buffer);
            self.res.table.buffers.set(id, size);
            Ok(id)
        })
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> u32 {
        autoreleasepool(|_| {
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let size = raw.length() as u64;
            let id = self.res.buffers.insert(raw);
            self.res.table.buffers.set(id, size);
            self.res.imports.insert(id);
            id
        })
    }

    pub fn raw_buffer(&self, buffer: u32) -> &ProtocolObject<dyn MTLBuffer> {
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer)
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.res.table.drop_buffer(buffer);
        self.res.imports.remove(&buffer);
        let buffer = self.res.buffers.remove(buffer);
        self.residency.remove(ProtocolObject::from_ref(&*buffer));
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
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

    pub fn unmap(&self, buffer: u32) {
        self.res.table.buffers.get(buffer);
    }

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        self.res.table.buffers.get(buffer);
        Ok(self.res.buffers.get(buffer).gpuAddress())
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.table.buffers.get(buffer);
        self.caps.address();
        self.res.buffers.get(buffer).gpuAddress()
    }
}
