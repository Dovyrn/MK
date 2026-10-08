use super::Device;
use crate::Buffer;
use crate::memory;
use crate::vk::buffer::BufferObject;
use ash::vk;
use std::rc::Rc;

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<Buffer, String> {
        self.caps.check_buffer(size, memory)?;
        let buffer = self.allocate(size, memory, self.buffer_usage())?;
        Ok(self.insert_buffer(buffer, size))
    }

    pub(super) fn allocate(
        &self,
        size: u64,
        memory: u32,
        usage: vk::BufferUsageFlags,
    ) -> Result<BufferObject, String> {
        let info = self.buffer_info(size, usage);
        let raw = unsafe { self.raw.create_buffer(&info, None) }.map_err(|e| e.to_string())?;
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
            device: memory == memory::DEVICE,
        };
        buffer.data = self
            .map_memory(*buffer.memory, index)
            .inspect_err(|_| buffer.free(self))?;
        let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
        buffer.address = unsafe { self.raw.get_buffer_device_address(&info) };
        Ok(buffer)
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer, size: u64) -> Buffer {
        let id = self.res.buffers.insert(BufferObject {
            raw,
            memory: Rc::new(vk::DeviceMemory::null()),
            address: 0,
            data: std::ptr::null_mut(),
            own: false,
            device: false,
        });
        self.res.table.buffers.set(id, size);
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

    pub fn map_buffer(&self, buffer: Buffer) -> *mut u8 {
        self.res.table.buffers.get(buffer);
        let b = self.res.buffers.get(buffer);
        assert!(b.own, "map of an imported buffer");
        assert!(!b.device, "map of a device buffer");
        b.data
    }

    pub fn unmap_buffer(&self, buffer: Buffer) {
        self.res.table.buffers.get(buffer);
    }

    pub(super) fn insert_buffer(&mut self, buffer: BufferObject, size: u64) -> Buffer {
        let slot = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(slot, size);
        slot
    }

    pub fn buffer_address(&self, buffer: Buffer) -> u64 {
        self.res.table.buffers.get(buffer);
        self.caps.assert_addresses();
        let b = self.res.buffers.get(buffer);
        assert!(b.own, "imported buffers have no address on Vulkan");
        b.address
    }
}
