use super::Device;
use crate::memory;
use crate::vk::buffer::Buffer;
use ash::vk;

impl Device {
    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps().buffer(size, memory)?;
        let buffer = self.allocate(size, memory, self.usage())?;
        Ok(self.insert_buffer(buffer, size))
    }

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        self.caps().buffer(size, memory::UPLOAD)?;
        let host = self
            .ext
            .host
            .as_ref()
            .ok_or("VK_EXT_external_memory_host is not enabled")?;
        let kind = vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT;
        let mut properties = vk::MemoryHostPointerPropertiesEXT::default();
        let result = unsafe {
            (host.fp().get_memory_host_pointer_properties_ext)(
                self.raw.handle(),
                kind,
                pointer.cast(),
                &mut properties,
            )
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let mut external = vk::ExternalMemoryBufferCreateInfo::default().handle_types(kind);
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .push_next(&mut external);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let bits = needs.memory_type_bits & properties.memory_type_bits;
            let Some(index) = self.find_type(bits, vk::MemoryPropertyFlags::HOST_VISIBLE) else {
                self.raw.destroy_buffer(raw, None);
                return Err("no host visible memory type fits this pointer".into());
            };
            let mut import = vk::ImportMemoryHostPointerInfoEXT::default()
                .handle_type(kind)
                .host_pointer(pointer.cast());
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(size)
                .memory_type_index(index)
                .push_next(&mut import)
                .push_next(&mut flags);
            let memory = match self.raw.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e.to_string());
                }
            };
            if let Err(e) = self.raw.bind_buffer_memory(raw, memory, 0) {
                self.raw.destroy_buffer(raw, None);
                self.raw.free_memory(memory, None);
                return Err(e.to_string());
            }
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory,
                    address,
                    data: pointer,
                    own: true,
                    device: false,
                },
                size,
            ))
        }
    }

    pub(super) fn allocate(
        &self,
        size: u64,
        memory: u32,
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer, String> {
        let device = memory == memory::DEVICE;
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let info = self.buffer_info(size, usage, &mut wide);
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let index = match self.memory_type(needs.memory_type_bits, memory) {
                Ok(i) => i,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e);
                }
            };
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut flags)
                .push_next(&mut dedicated);
            let memory = match self.raw.allocate_memory(&info, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e.to_string());
                }
            };
            let mut buffer = Buffer {
                raw,
                memory,
                address: 0,
                data: std::ptr::null_mut(),
                own: true,
                device,
            };
            if let Err(e) = self.raw.bind_buffer_memory(raw, memory, 0) {
                buffer.free(&self.raw);
                return Err(e.to_string());
            }
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            if visible {
                match self
                    .raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                {
                    Ok(data) => buffer.data = data.cast(),
                    Err(e) => {
                        buffer.free(&self.raw);
                        return Err(e.to_string());
                    }
                }
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            buffer.address = self.raw.get_buffer_device_address(&info);
            Ok(buffer)
        }
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer) -> u32 {
        let id = self.res.buffers.insert(Buffer {
            raw,
            memory: vk::DeviceMemory::null(),
            address: 0,
            data: std::ptr::null_mut(),
            own: false,
            device: false,
        });
        self.res.table.buffers.set(id, u64::MAX);
        id
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.res.table.buffers.clear(buffer);
        self.defer(move |d| {
            let buffer = d.res.buffers.remove(buffer);
            buffer.free(&d.raw);
        });
    }

    pub fn raw_buffer(&self, buffer: u32) -> vk::Buffer {
        self.res.buffers.get(buffer).raw
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        let b = self.res.buffers.get(buffer);
        assert!(!b.device, "map of a device buffer");
        b.data
    }

    pub fn unmap(&self, _buffer: u32) {}

    pub(super) fn insert_buffer(&mut self, buffer: Buffer, size: u64) -> u32 {
        let limits = unsafe {
            self.instance
                .get_physical_device_properties(self.physical)
                .limits
        };
        let info = vk::DescriptorBufferInfo::default()
            .buffer(buffer.raw)
            .range(size.min(limits.max_storage_buffer_range as u64));
        let slot = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(slot, size);
        if slot < self.bindless.buffers {
            self.bindless.write_buffer(&self.raw, slot, info);
        }
        slot
    }

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        if buffer >= self.bindless.buffers || !self.res.buffers.get(buffer).own {
            return Err("the bindless heap has no slot for this buffer".into());
        }
        Ok((self.bindless.images + buffer) as u64)
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).address
    }
}
