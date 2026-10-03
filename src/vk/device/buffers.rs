use super::Device;
use crate::memory;
use crate::vk::buffer::Buffer;
use ash::vk;
use std::rc::Rc;

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps.buffer(size, memory)?;
        let buffer = self.allocate(size, memory, self.usage())?;
        Ok(self.insert_buffer(buffer, size))
    }

    pub(super) fn allocate(
        &self,
        size: u64,
        memory: u32,
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer, String> {
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
        let mut buffer = Buffer {
            raw,
            memory: Rc::new(
                self.attach(index, alloc, |m| unsafe {
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
            .mapped(*buffer.memory, index)
            .inspect_err(|_| buffer.free(self))?;
        let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
        buffer.address = unsafe { self.raw.get_buffer_device_address(&info) };
        Ok(buffer)
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer, size: u64) -> u32 {
        let id = self.res.buffers.insert(Buffer {
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

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.res.table.buffers.clear(buffer);
        self.defer(move |d| {
            let buffer = d.res.buffers.remove(buffer);
            buffer.free(d);
        });
    }

    pub fn raw_buffer(&self, buffer: u32) -> vk::Buffer {
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer).raw
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.res.table.buffers.get(buffer);
        let b = self.res.buffers.get(buffer);
        assert!(b.own, "map of an imported buffer");
        assert!(!b.device, "map of a device buffer");
        b.data
    }

    pub fn unmap(&self, buffer: u32) {
        self.res.table.buffers.get(buffer);
    }

    pub(super) fn insert_buffer(&mut self, buffer: Buffer, size: u64) -> u32 {
        let info = vk::DescriptorBufferInfo::default()
            .buffer(buffer.raw)
            .range(size.min(self.limits.max_storage_buffer_range as u64));
        let slot = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(slot, size);
        if slot < self.bindless.buffers {
            self.bindless.write_buffer(&self.raw, slot, info);
        }
        slot
    }

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        self.address(buffer);
        if buffer >= self.bindless.buffers {
            return Err("the bindless heap has no slot for this buffer".into());
        }
        Ok(self.bindless.buffer(buffer) as u64)
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.table.buffers.get(buffer);
        self.caps.address();
        let b = self.res.buffers.get(buffer);
        assert!(b.own, "imported buffers have no address on Vulkan");
        b.address
    }
}
