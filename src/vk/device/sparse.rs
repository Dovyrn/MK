use super::{Device, image_info, whole};
use crate::vk::buffer::Buffer;
use crate::{SparseRange, SparseRegion, TextureInfo, dimension, memory};
use ash::vk;

impl Device {
    pub fn sparse_page(&self) -> Result<u64, String> {
        if !self.caps.sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(1, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let probe = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let alignment = self.raw.get_buffer_memory_requirements(probe).alignment;
            self.raw.destroy_buffer(probe, None);
            Ok(alignment)
        }
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        self.texture(info)?;
        if !self.caps.sparse_textures {
            return Err("sparse textures are not supported".into());
        }
        let f = &self.ext.features;
        let sparse = match (info.dimension, info.samples) {
            (dimension::D1 | dimension::D1_ARRAY, _) => 0,
            (dimension::D3, _) => f.sparse_residency_image3_d,
            (_, 1) => f.sparse_residency_image2_d,
            (_, 2) => f.sparse_residency2_samples,
            (_, 4) => f.sparse_residency4_samples,
            (_, 8) => f.sparse_residency8_samples,
            (_, 16) => f.sparse_residency16_samples,
            _ => 0,
        };
        if sparse == 0 {
            return Err(
                "sparse residency is not supported for this dimension and sample count".into(),
            );
        }
        let create = image_info(info);
        let formats = unsafe {
            self.instance
                .get_physical_device_sparse_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.samples,
                    create.usage,
                    vk::ImageTiling::OPTIMAL,
                )
        };
        let f = formats
            .first()
            .ok_or(format!("format {} has no sparse layout", info.format))?;
        let g = f.image_granularity;
        Ok([g.width, g.height, g.depth])
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        self.caps.buffer(size, memory::DEVICE)?;
        if !self.caps.sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(
                Buffer {
                    raw,
                    memory: vk::DeviceMemory::null(),
                    address,
                    data: std::ptr::null_mut(),
                    own: true,
                    device: false,
                },
                size,
            ))
        }
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        let [width, height, depth] = self.sparse_tile(info)?;
        let create = image_info(info);
        let create = create.flags(
            create.flags
                | vk::ImageCreateFlags::SPARSE_BINDING
                | vk::ImageCreateFlags::SPARSE_RESIDENCY,
        );
        let image = unsafe {
            self.raw
                .create_image(&self.share_image(create), None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.register(image, vk::DeviceMemory::null(), true, &whole(info))?;
        self.res.table.textures.set(slot, *info);
        self.res.textures.get_mut(slot).tile = vk::Extent3D {
            width,
            height,
            depth,
        };
        Ok(slot)
    }

    fn bind_sparse(
        &mut self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        buffers: &[vk::SparseBufferMemoryBindInfo],
        images: &[vk::SparseImageMemoryBindInfo],
    ) -> Result<(), String> {
        self.res.table.submit(queue, waits, signals);
        let q = &self.queues[queue];
        let value = q.count + 1;
        let wait: Vec<_> = waits.iter().map(|w| self.res.events.get(w.0).raw).collect();
        let wait_values: Vec<_> = waits.iter().map(|w| w.1).collect();
        let mut signal: Vec<_> = signals
            .iter()
            .map(|s| self.res.events.get(s.0).raw)
            .collect();
        let mut signal_values: Vec<_> = signals.iter().map(|s| s.1).collect();
        signal.push(q.semaphore);
        signal_values.push(value);
        let mut timeline = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&wait_values)
            .signal_semaphore_values(&signal_values);
        let info = vk::BindSparseInfo::default()
            .wait_semaphores(&wait)
            .buffer_binds(buffers)
            .image_binds(images)
            .signal_semaphores(&signal)
            .push_next(&mut timeline);
        unsafe {
            self.raw
                .queue_bind_sparse(q.raw, &[info], vk::Fence::null())
                .map_err(|e| e.to_string())?
        };
        self.queues[queue].count = value;
        Ok(())
    }

    pub fn map_sparse_buffer(
        &mut self,
        queue: usize,
        buffer: u32,
        heap: u32,
        ranges: &[SparseRange],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let binds: Vec<_> = ranges
            .iter()
            .map(|r| vk::SparseMemoryBind {
                resource_offset: r.page as u64 * page,
                size: r.count as u64 * page,
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseBufferMemoryBindInfo::default()
            .buffer(self.res.buffers.get(buffer).raw)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &infos, &[])
    }

    pub fn map_sparse_texture(
        &mut self,
        queue: usize,
        texture: u32,
        heap: u32,
        regions: &[SparseRegion],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let t = self.res.textures.get(texture);
        let binds: Vec<_> = regions
            .iter()
            .map(|r| vk::SparseImageMemoryBind {
                subresource: vk::ImageSubresource {
                    aspect_mask: t.aspect,
                    mip_level: r.mip,
                    array_layer: r.layer,
                },
                offset: vk::Offset3D {
                    x: (r.x * t.tile.width) as i32,
                    y: (r.y * t.tile.height) as i32,
                    z: (r.z * t.tile.depth) as i32,
                },
                extent: vk::Extent3D {
                    width: r.width * t.tile.width,
                    height: r.height * t.tile.height,
                    depth: r.depth * t.tile.depth,
                },
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseImageMemoryBindInfo::default()
            .image(t.image)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &[], &infos)
    }
}
