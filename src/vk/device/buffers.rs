use super::Device;
use crate::Error;
use crate::table::lock;
use crate::vk::buffer::BufferObject;
use crate::{Buffer, BufferUsage};
use ash::vk;
use std::sync::Arc;

fn vk_buffer_usage(usage: BufferUsage) -> vk::BufferUsageFlags {
    let map = [
        (BufferUsage::UNIFORM, vk::BufferUsageFlags::UNIFORM_BUFFER),
        (BufferUsage::STORAGE, vk::BufferUsageFlags::STORAGE_BUFFER),
        (BufferUsage::VERTEX, vk::BufferUsageFlags::VERTEX_BUFFER),
        (BufferUsage::INDEX, vk::BufferUsageFlags::INDEX_BUFFER),
        (BufferUsage::INDIRECT, vk::BufferUsageFlags::INDIRECT_BUFFER),
        (BufferUsage::COPY_SRC, vk::BufferUsageFlags::TRANSFER_SRC),
        (BufferUsage::COPY_DST, vk::BufferUsageFlags::TRANSFER_DST),
        (
            BufferUsage::ADDRESS,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        ),
    ];
    let mut bits = vk::BufferUsageFlags::empty();
    for (u, f) in map {
        if usage.contains(u) {
            bits |= f;
        }
    }
    bits
}

impl Device {
    pub fn create_buffer(
        &self,
        size: u64,
        memory: crate::Memory,
        usage: BufferUsage,
        label: &str,
    ) -> Result<Buffer, Error> {
        self.caps.check_buffer(size, memory, usage)?;
        let buffer = self.allocate(size, memory, usage)?;
        self.name(buffer.raw, label);
        let mappable = !buffer.data.is_null();
        let id = self.res.buffers.insert(buffer);
        self.res.table.add_buffer(id, size, mappable, usage);
        Ok(id)
    }

    fn allocate(
        &self,
        size: u64,
        memory: crate::Memory,
        usage: BufferUsage,
    ) -> Result<BufferObject, Error> {
        let address = usage.contains(BufferUsage::ADDRESS);
        let info = self.buffer_info(size, vk_buffer_usage(usage));
        let raw = unsafe { self.raw.create_buffer(&info, None) }.map_err(Error::from)?;
        let destroy = || unsafe { self.raw.destroy_buffer(raw, None) };
        let needs = unsafe { self.raw.get_buffer_memory_requirements(raw) };
        let index = self
            .memory_type(needs.memory_type_bits, memory)
            .inspect_err(|_| destroy())?;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
        let mut alloc = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .push_next(&mut dedicated);
        if address {
            alloc = alloc.push_next(&mut flags);
        }
        let mut buffer = BufferObject {
            raw,
            memory: Arc::new(
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
        if address {
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            buffer.address = unsafe { self.raw.get_buffer_device_address(&info) };
        }
        Ok(buffer)
    }

    pub fn import_buffer(
        &self,
        raw: vk::Buffer,
        size: u64,
        usage: BufferUsage,
        data: *mut u8,
    ) -> Buffer {
        let id = self.res.buffers.insert(BufferObject {
            raw,
            memory: Arc::new(vk::DeviceMemory::null()),
            address: 0,
            data,
            own: false,
        });
        self.res.table.add_buffer(id, size, !data.is_null(), usage);
        lock(&self.res.table.residency).exempt_buffer(id);
        id
    }

    pub fn destroy_buffer(&self, buffer: Buffer) {
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

    pub fn buffer_address(&self, buffer: Buffer) -> u64 {
        let name = "buffer_address";
        self.res
            .table
            .assert_usage(name, buffer, BufferUsage::ADDRESS, "ADDRESS");
        self.caps.assert_addresses();
        let b = self.res.buffers.get(buffer);
        if b.own {
            return b.address;
        }
        let info = vk::BufferDeviceAddressInfo::default().buffer(b.raw);
        unsafe { self.raw.get_buffer_device_address(&info) }
    }
}
