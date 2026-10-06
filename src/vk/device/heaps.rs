use super::{Device, image_info, whole};
use crate::vk::buffer::Buffer;
use crate::vk::heap::Heap;
use crate::{TextureInfo, format, memory, usage};
use ash::vk;
use std::rc::Rc;

impl Device {
    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.heap(size, memory)
    }

    fn heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps.heap(size, memory);
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
        let mut bits = self.buffer_needs(4096).memory_type_bits;
        if device {
            bits &= self.texture_needs(&color).memory_type_bits
                & self.texture_needs(&depth).memory_type_bits;
        }
        let index = self.memory_type(bits, memory)?;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut flags);
        let memory = self.alloc(&info)?;
        let data = self
            .mapped(memory, index)
            .inspect_err(|_| self.release(memory))?;
        let id = self.res.heaps.insert(Heap {
            memory: Rc::new(memory),
            data,
            device,
            index,
        });
        self.res.table.heaps.set(id, size);
        Ok(id)
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        self.res.table.drop_heap(heap);
        let heap = self.res.heaps.remove(heap);
        if Rc::strong_count(&heap.memory) == 1 {
            self.release(*heap.memory);
        }
    }

    fn buffer_needs(&self, size: u64) -> vk::MemoryRequirements {
        let info = self.buffer_info(size, self.usage());
        let query = vk::DeviceBufferMemoryRequirements::default().create_info(&info);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_buffer_memory_requirements(&query, &mut needs)
        };
        needs.memory_requirements
    }

    fn texture_needs(&self, info: &TextureInfo) -> vk::MemoryRequirements {
        let mut image = self.share_image(image_info(info));
        if info.usage & usage::HOST != 0 {
            image.usage |= vk::ImageUsageFlags::HOST_TRANSFER_EXT;
        }
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
        let scratch = if self.caps.ray { 256 } else { 1 };
        (needs.size, needs.alignment.max(scratch))
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let needs = self.texture_needs(info);
        let page = self.limits.buffer_image_granularity;
        (needs.size.next_multiple_of(page), needs.alignment.max(page))
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        self.res.table.heaps.get(heap);
        self.caps.buffer(size, memory::DEVICE)?;
        self.res
            .table
            .place(heap, offset, self.buffer_size(size, memory::DEVICE));
        let h = self.res.heaps.get(heap);
        unsafe {
            let info = self.buffer_info(size, self.usage());
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
            if let Err(e) = self.raw.bind_buffer_memory(raw, *h.memory, offset) {
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
            let id = self.insert_buffer(
                Buffer {
                    raw,
                    memory: h.memory.clone(),
                    address,
                    data,
                    own: true,
                    device: h.device,
                },
                size,
            );
            self.res.table.buffer_heaps.set(id, heap);
            Ok(id)
        }
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        self.res.table.heaps.get(heap);
        crate::caps::attached(info.memory);
        self.texture(info)?;
        self.res.table.place(heap, offset, self.texture_size(info));
        let h = self.res.heaps.get(heap);
        let (memory, index) = (h.memory.clone(), h.index);
        let mut create = self.share_image(image_info(info));
        self.host_usage(info, &mut create)?;
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
            if let Err(e) = self.raw.bind_image_memory(image, *memory, offset) {
                self.raw.destroy_image(image, None);
                return Err(e.to_string());
            }
            let id = self.register(image, Some(Rc::new(vk::DeviceMemory::null())), &whole(info))?;
            let t = self.res.textures.get_mut(id);
            t.heap = Some(memory);
            self.res.table.textures.set(id, *info);
            self.res.table.texture_heaps.set(id, heap);
            self.prepare(id)
        }
    }
}
