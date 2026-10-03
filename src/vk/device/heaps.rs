use super::{Device, image_info, whole};
use crate::vk::buffer::Buffer;
use crate::vk::heap::Heap;
use crate::{TextureInfo, format, memory, usage};
use ash::vk;
use std::rc::Rc;

const FD: &str = "VK_KHR_external_memory_fd is not enabled";

impl Device {
    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.heap(size, memory, false, None)
    }

    pub fn create_shared_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fd.as_ref().ok_or(FD)?;
        self.heap(size, memory, true, None)
    }

    pub fn import_heap(&mut self, fd: i32, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fd.as_ref().ok_or(FD)?;
        self.heap(size, memory, true, Some(fd))
    }

    pub fn export_heap(&self, heap: u32) -> Result<i32, String> {
        self.res.table.heaps.get(heap);
        let fd = self.ext.fd.as_ref().ok_or(FD)?;
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.res.heaps.get(heap).memory)
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        unsafe { fd.get_memory_fd(&info).map_err(|e| e.to_string()) }
    }

    fn heap(
        &mut self,
        size: u64,
        memory: u32,
        shared: bool,
        fd: Option<i32>,
    ) -> Result<u32, String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        let device = memory == memory::DEVICE;
        let color = TextureInfo {
            format: format::RGBA8_UNORM,
            usage: usage::SAMPLED
                | usage::STORAGE
                | usage::COLOR
                | usage::COPY_SRC
                | usage::COPY_DST,
            ..Default::default()
        };
        let depth = TextureInfo {
            format: format::D32_FLOAT,
            usage: usage::SAMPLED | usage::DEPTH | usage::COPY_SRC | usage::COPY_DST,
            ..Default::default()
        };
        let bits = self.buffer_needs(4096).memory_type_bits
            & self.texture_needs(&color).memory_type_bits
            & self.texture_needs(&depth).memory_type_bits;
        let index = self
            .memory_type(bits, memory)
            .or_else(|_| self.memory_type(!0, memory))?;
        let kind = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(kind);
        let mut import = vk::ImportMemoryFdInfoKHR::default()
            .handle_type(kind)
            .fd(fd.unwrap_or(-1));
        let mut info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut flags);
        if fd.is_some() {
            info = info.push_next(&mut import);
        } else if shared {
            info = info.push_next(&mut export);
        }
        let memory = self.alloc(&info)?;
        let data = self
            .mapped(memory, index)
            .inspect_err(|_| self.release(memory))?;
        let id = self.res.heaps.insert(Heap {
            memory,
            data,
            shared,
            device,
            index,
        });
        self.res.table.heaps.set(id, size);
        Ok(id)
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        self.defer(move |d| {
            let heap = d.res.heaps.remove(heap);
            d.release(heap.memory);
        });
    }

    fn buffer_needs(&self, size: u64) -> vk::MemoryRequirements {
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self.buffer_info(size, self.usage(), &mut wide);
        let query = vk::DeviceBufferMemoryRequirements::default().create_info(&info);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_buffer_memory_requirements(&query, &mut needs)
        };
        needs.memory_requirements
    }

    fn texture_needs(&self, info: &TextureInfo) -> vk::MemoryRequirements {
        let image = self.share_image(image_info(info));
        let query = vk::DeviceImageMemoryRequirements::default().create_info(&image);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_image_memory_requirements(&query, &mut needs)
        };
        needs.memory_requirements
    }

    pub fn buffer_size(&self, size: u64, _memory: u32) -> (u64, u64) {
        let needs = self.buffer_needs(size);
        (needs.size, needs.alignment)
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let needs = self.texture_needs(info);
        (needs.size, needs.alignment)
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        self.res.table.heaps.get(heap);
        self.caps.buffer(size, memory::DEVICE)?;
        let h = self.res.heaps.get(heap);
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let mut external = vk::ExternalMemoryBufferCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let mut info = self.buffer_info(size, self.usage(), &mut wide);
            if h.shared {
                info = info.push_next(&mut external);
            }
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let bits = self
                .raw
                .get_buffer_memory_requirements(raw)
                .memory_type_bits;
            if bits & (1 << h.index) == 0 {
                self.raw.destroy_buffer(raw, None);
                return Err("the heap's memory type does not fit this buffer".into());
            }
            if let Err(e) = self.raw.bind_buffer_memory(raw, h.memory, offset) {
                self.raw.destroy_buffer(raw, None);
                return Err(e.to_string());
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            let address = self.raw.get_buffer_device_address(&info);
            let data = if h.data.is_null() {
                h.data
            } else {
                h.data.add(offset as usize)
            };
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory: vk::DeviceMemory::null(),
                    address,
                    data,
                    own: true,
                    device: h.device,
                },
                size,
            ))
        }
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        self.res.table.heaps.get(heap);
        self.texture(info)?;
        let h = self.res.heaps.get(heap);
        let (memory, index) = (h.memory, h.index);
        let mut external = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let mut create = self.share_image(image_info(info));
        if h.shared {
            create = create.push_next(&mut external);
        }
        unsafe {
            let image = self
                .raw
                .create_image(&create, None)
                .map_err(|e| e.to_string())?;
            let bits = self
                .raw
                .get_image_memory_requirements(image)
                .memory_type_bits;
            if bits & (1 << index) == 0 {
                self.raw.destroy_image(image, None);
                return Err("the heap's memory type does not fit this texture".into());
            }
            if let Err(e) = self.raw.bind_image_memory(image, memory, offset) {
                self.raw.destroy_image(image, None);
                return Err(e.to_string());
            }
            let id = self.register(image, Some(Rc::new(vk::DeviceMemory::null())), &whole(info))?;
            self.res.table.textures.set(id, *info);
            Ok(id)
        }
    }
}
