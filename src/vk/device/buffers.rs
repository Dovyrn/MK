use super::Device;
use crate::Buffer;
use crate::Error;
use crate::vk::buffer::BufferObject;
use ash::vk;
use std::rc::Rc;

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: crate::Memory) -> Result<Buffer, Error> {
        self.caps.check_buffer(size, memory)?;
        let buffer = self.allocate(size, memory, self.buffer_usage())?;
        Ok(self.insert_buffer(buffer, size))
    }

    pub(super) fn allocate(
        &self,
        size: u64,
        memory: crate::Memory,
        usage: vk::BufferUsageFlags,
    ) -> Result<BufferObject, Error> {
        let info = self.buffer_info(size, usage);
        let raw = unsafe { self.raw.create_buffer(&info, None) }.map_err(Error::from)?;
        let destroy = || unsafe { self.raw.destroy_buffer(raw, None) };
        let needs = unsafe { self.raw.get_buffer_memory_requirements(raw) };
        let index = self
            .memory_type(needs.memory_type_bits, memory)
            .inspect_err(|_| destroy())?;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
        let alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .push_next(&mut flags)
            .push_next(&mut dedicated);
        let mut buffer = BufferObject {
            raw,
            memory: Rc::new(
                self.allocate_bound(index, alloc, |m| unsafe {
                    self.raw.bind_buffer_memory(raw, m, 0)
                })
                .inspect_err(|_| destroy())?,
            ),
            address: 0,
            data: std::ptr::null_mut(),
            own: true,
        };
        buffer.data = self
            .map_memory(*buffer.memory, index)
            .inspect_err(|_| buffer.free(self))?;
        let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
        buffer.address = unsafe { self.raw.get_buffer_device_address(&info) };
        Ok(buffer)
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer, size: u64, data: *mut u8) -> Buffer {
        let id = self.res.buffers.insert(BufferObject {
            raw,
            memory: Rc::new(vk::DeviceMemory::null()),
            address: 0,
            data,
            own: false,
        });
        self.res.table.add_buffer(id, size, !data.is_null());
        self.res.table.residency.exempt_buffer(id);
        id
    }

    pub fn destroy_buffer(&mut self, buffer: Buffer) {
        self.res.table.drop_buffer(buffer);
        let buffer = self.res.buffers.remove(buffer);
        buffer.free(self);
    }

    pub fn raw_buffer(&self, buffer: Buffer) -> vk::Buffer {
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer).raw
    }

    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    pub unsafe fn map_buffer<T: Copy>(&self, buffer: Buffer) -> &mut [T] {
        let b = self.res.buffers.get(buffer);
        unsafe { self.res.table.mapping(buffer, b.data) }
    }

    pub fn unmap_buffer(&self, buffer: Buffer) {
        self.res.table.buffers.get(buffer);
    }

    pub(super) fn insert_buffer(&mut self, buffer: BufferObject, size: u64) -> Buffer {
        let mappable = !buffer.data.is_null();
        let slot = self.res.buffers.insert(buffer);
        self.res.table.add_buffer(slot, size, mappable);
        slot
    }

    pub fn buffer_address(&self, buffer: Buffer) -> u64 {
        self.res.table.buffers.get(buffer);
        self.caps.assert_addresses();
        let b = self.res.buffers.get(buffer);
        if b.own {
            return b.address;
        }
        let info = vk::BufferDeviceAddressInfo::default().buffer(b.raw);
        unsafe { self.raw.get_buffer_device_address(&info) }
    }
}
