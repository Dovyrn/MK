use super::{Device, options};
use crate::memory;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLDevice, MTLResource, MTLStorageMode};
use std::ptr::NonNull;

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps.buffer(size, memory)?;
        let buffer = self
            .raw
            .newBufferWithLength_options(size as usize, options(memory))
            .ok_or("buffer allocation failed")?;
        self.residency.add(ProtocolObject::from_ref(&*buffer));
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        self.caps.buffer(size, memory::UPLOAD)?;
        let buffer = unsafe {
            self.raw
                .newBufferWithBytesNoCopy_length_options_deallocator(
                    NonNull::new(pointer.cast()).ok_or("null pointer")?,
                    size as usize,
                    options(memory::READBACK),
                    None,
                )
                .ok_or("host buffer")?
        };
        self.residency.add(ProtocolObject::from_ref(&*buffer));
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn import_buffer(&mut self, raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> u32 {
        self.residency.add(ProtocolObject::from_ref(&*raw));
        let size = raw.length() as u64;
        let id = self.res.buffers.insert(raw);
        self.res.table.buffers.set(id, size);
        id
    }

    pub fn raw_buffer(&self, buffer: u32) -> &ProtocolObject<dyn MTLBuffer> {
        self.res.buffers.get(buffer)
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.res.table.buffers.clear(buffer);
        let buffer = self.res.buffers.remove(buffer);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*buffer)));
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        let b = self.res.buffers.get(buffer);
        assert!(
            b.storageMode() != MTLStorageMode::Private,
            "map of a device buffer"
        );
        b.contents().as_ptr().cast()
    }

    pub fn unmap(&self, _buffer: u32) {}

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        Ok(self.res.buffers.get(buffer).gpuAddress())
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).gpuAddress()
    }
}
