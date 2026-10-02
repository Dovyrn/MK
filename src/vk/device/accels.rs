use super::Device;
use crate::vk::accel::Accel;
use crate::{AccelInfo, format, index, memory};
use ash::vk;

impl Device {
    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let (kind, geometry, ranges) = if !info.boxes.is_empty() {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for b in info.boxes {
                let aabbs = vk::AccelerationStructureGeometryAabbsDataKHR::default()
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: self.res.buffers.get(b.buffer).address + b.offset,
                    })
                    .stride(b.stride as u64);
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::AABBS)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { aabbs })
                        .flags(if b.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(b.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        } else if info.triangles.is_empty() {
            let instances = vk::AccelerationStructureGeometryInstancesDataKHR::default().data(
                vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(info.instances).address
                        + info.instance_offset,
                },
            );
            let geometry = vk::AccelerationStructureGeometryKHR::default()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(vk::AccelerationStructureGeometryDataKHR { instances });
            let range = vk::AccelerationStructureBuildRangeInfoKHR::default()
                .primitive_count(info.instance_count);
            (
                vk::AccelerationStructureTypeKHR::TOP_LEVEL,
                vec![geometry],
                vec![range],
            )
        } else {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for t in info.triangles {
                let address = |b: u32, o: u64| vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(b).address + o,
                };
                let mut triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
                    .vertex_format(format::vk(t.format))
                    .vertex_data(address(t.vertices, t.vertex_offset))
                    .vertex_stride(t.stride as u64)
                    .max_vertex(t.vertex_count.saturating_sub(1))
                    .index_type(vk::IndexType::NONE_KHR);
                if t.indices != !0 {
                    triangles = triangles
                        .index_type(match t.index_kind {
                            index::U16 => vk::IndexType::UINT16,
                            index::U32 => vk::IndexType::UINT32,
                            k => panic!("unknown index {k}"),
                        })
                        .index_data(address(t.indices, t.index_offset));
                }
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
                        .flags(if t.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(t.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        };
        let flags = vk::BuildAccelerationStructureFlagsKHR::from_raw(info.flags);
        let build = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(kind)
            .flags(flags)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometry);
        let counts: Vec<_> = ranges.iter().map(|r| r.primitive_count).collect();
        let mut sizes = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            ray.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build,
                &counts,
                &mut sizes,
            )
        };
        let buffer = self.allocate(
            sizes.acceleration_structure_size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(sizes.acceleration_structure_size)
            .ty(kind);
        let raw = match unsafe { ray.create_acceleration_structure(&create, None) } {
            Ok(r) => r,
            Err(e) => {
                self.free_buffer(&buffer);
                return Err(e.to_string());
            }
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry,
            ranges,
            build: sizes.build_scratch_size,
            update: sizes.update_scratch_size,
        }))
    }

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let source = self.res.accels.get(accel);
        let kind = source.kind;
        let flags = source.flags;
        let buffer = self.allocate(
            size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(size)
            .ty(kind);
        let raw = match unsafe { ray.create_acceleration_structure(&create, None) } {
            Ok(r) => r,
            Err(e) => {
                self.free_buffer(&buffer);
                return Err(e.to_string());
            }
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry: Vec::new(),
            ranges: Vec::new(),
            build: 0,
            update: 0,
        }))
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        self.defer(move |d| {
            let a = d.res.accels.remove(accel);
            unsafe {
                d.ext
                    .ray
                    .as_ref()
                    .unwrap()
                    .destroy_acceleration_structure(a.raw, None);
                d.raw.destroy_buffer(a.buffer.raw, None);
                d.raw.free_memory(a.buffer.memory, None);
            }
        });
    }

    pub fn accel_scratch(&self, accel: u32) -> (u64, u64) {
        let a = self.res.accels.get(accel);
        (a.build, a.update)
    }

    pub fn accel_handle(&self, accel: u32) -> u64 {
        self.res.accels.get(accel).address
    }

    pub fn write_instance(
        &self,
        out: &mut [u8],
        accel: u32,
        transform: &[f32; 12],
        id: u32,
        mask: u32,
        flags: u32,
    ) {
        for (i, v) in transform.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out[48..52].copy_from_slice(&(id & 0xff_ffff | mask << 24).to_le_bytes());
        out[52..56].copy_from_slice(&(flags << 24).to_le_bytes());
        out[56..64].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }
}
