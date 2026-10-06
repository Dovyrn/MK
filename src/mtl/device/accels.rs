use super::{Device, id, vertex_format};
use crate::mtl::accel::Accel;
use crate::{AccelInfo, accel, index};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSArray;
use objc2_metal::{
    MTL4AccelerationStructureBoundingBoxGeometryDescriptor, MTL4AccelerationStructureDescriptor,
    MTL4AccelerationStructureGeometryDescriptor,
    MTL4AccelerationStructureTriangleGeometryDescriptor, MTL4BufferRange,
    MTL4InstanceAccelerationStructureDescriptor, MTL4PrimitiveAccelerationStructureDescriptor,
    MTLAccelerationStructure, MTLAccelerationStructureInstanceDescriptorType,
    MTLAccelerationStructureUsage, MTLAttributeFormat, MTLBuffer, MTLDevice, MTLIndexType,
    MTLMatrixLayout,
};

impl Device {
    fn range(&self, buffer: u32, offset: u64, length: u64) -> MTL4BufferRange {
        self.res.table.buffers.get(buffer);
        MTL4BufferRange {
            bufferAddress: self.res.buffers.get(buffer).gpuAddress() + offset,
            length,
        }
    }

    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        self.caps.accel(info);
        autoreleasepool(|_| {
            let desc: Retained<MTL4AccelerationStructureDescriptor> = if !info.boxes.is_empty() {
                let geometry: Vec<Retained<MTL4AccelerationStructureGeometryDescriptor>> = info
                    .boxes
                    .iter()
                    .map(|b| {
                        let g = MTL4AccelerationStructureBoundingBoxGeometryDescriptor::new();
                        g.setBoundingBoxBuffer(self.range(b.buffer, b.offset, !0));
                        unsafe {
                            g.setBoundingBoxStride(b.stride as usize);
                            g.setBoundingBoxCount(b.count as usize);
                        }
                        g.setOpaque(b.opaque);
                        Retained::into_super(g)
                    })
                    .collect();
                let d = MTL4PrimitiveAccelerationStructureDescriptor::new();
                d.setGeometryDescriptors(Some(&NSArray::from_retained_slice(&geometry)));
                Retained::into_super(d)
            } else if info.triangles.is_empty() {
                let (count, _) = *self.res.table.rays.get(info.ray_instances);
                let buffer = *self.res.rays.get(info.ray_instances);
                let d = MTL4InstanceAccelerationStructureDescriptor::new();
                unsafe {
                    d.setInstanceDescriptorBuffer(self.range(buffer, 0, count as u64 * 72));
                    d.setInstanceDescriptorStride(72);
                    d.setInstanceCount(count as usize);
                }
                d.setInstanceDescriptorType(
                    MTLAccelerationStructureInstanceDescriptorType::Indirect,
                );
                d.setInstanceTransformationMatrixLayout(MTLMatrixLayout::RowMajor);
                Retained::into_super(d)
            } else {
                let geometry: Vec<Retained<MTL4AccelerationStructureGeometryDescriptor>> = info
                    .triangles
                    .iter()
                    .map(|t| {
                        let g = MTL4AccelerationStructureTriangleGeometryDescriptor::new();
                        unsafe {
                            g.setVertexBuffer(self.range(t.vertices, t.vertex_offset, !0));
                            g.setVertexStride(t.stride as usize);
                            g.setTriangleCount(t.count as usize);
                        }
                        g.setVertexFormat(MTLAttributeFormat(vertex_format(t.format).0));
                        if t.indices != !0 {
                            unsafe { g.setIndexBuffer(self.range(t.indices, t.index_offset, !0)) };
                            g.setIndexType(match t.index_kind {
                                index::U16 => MTLIndexType::UInt16,
                                index::U32 => MTLIndexType::UInt32,
                                k => panic!("unknown index {k}"),
                            });
                        }
                        g.setOpaque(t.opaque);
                        Retained::into_super(g)
                    })
                    .collect();
                let d = MTL4PrimitiveAccelerationStructureDescriptor::new();
                d.setGeometryDescriptors(Some(&NSArray::from_retained_slice(&geometry)));
                Retained::into_super(d)
            };
            let mut usage = MTLAccelerationStructureUsage::None;
            for (bit, flag) in [
                (accel::UPDATE, MTLAccelerationStructureUsage::Refit),
                (
                    accel::FAST_BUILD,
                    MTLAccelerationStructureUsage::PreferFastBuild,
                ),
                (
                    accel::FAST_TRACE,
                    MTLAccelerationStructureUsage::PreferFastIntersection,
                ),
                (accel::SMALL, MTLAccelerationStructureUsage::MinimizeMemory),
            ] {
                if info.flags & bit != 0 {
                    usage |= flag;
                }
            }
            desc.setUsage(usage);
            let sizes = self.raw.accelerationStructureSizesWithDescriptor(&desc);
            let raw = self
                .raw
                .newAccelerationStructureWithSize(sizes.accelerationStructureSize)
                .ok_or("acceleration structure allocation failed")?;
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let id = self.res.accels.insert(Accel {
                raw,
                desc,
                build: sizes.buildScratchBufferSize as u64,
                update: sizes.refitScratchBufferSize as u64,
            });
            let scratch = (
                sizes.buildScratchBufferSize as u64,
                sizes.refitScratchBufferSize as u64,
            );
            let mut buffers = crate::table::geometry(info);
            if info.ray_instances != !0 {
                buffers.push(*self.res.rays.get(info.ray_instances));
            }
            self.res.table.accel(id, scratch, buffers);
            Ok(id)
        })
    }

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.res.table.accels.get(accel);
            let raw = self
                .raw
                .newAccelerationStructureWithSize(size as usize)
                .ok_or("acceleration structure allocation failed")?;
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let desc = self.res.accels.get(accel).desc.clone();
            let id = self.res.accels.insert(Accel {
                raw,
                desc,
                build: 0,
                update: 0,
            });
            self.res.table.accel(id, (0, 0), Vec::new());
            Ok(id)
        })
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        self.res.table.drop_accel(accel);
        let a = self.res.accels.remove(accel);
        self.residency.remove(ProtocolObject::from_ref(&*a.raw));
    }

    pub fn accel_scratch(&self, accel: u32) -> (u64, u64) {
        self.res.table.accels.get(accel);
        let a = self.res.accels.get(accel);
        (a.build, a.update)
    }

    pub fn accel_handle(&self, accel: u32) -> u64 {
        self.res.table.accels.get(accel);
        id(self.res.accels.get(accel).raw.gpuResourceID())
    }

    pub fn create_ray_instances(&mut self, count: u32, memory: u32) -> Result<u32, String> {
        assert!(count > 0, "ray instances count is 0");
        if !self.caps.ray {
            return Err("ray tracing is not supported".into());
        }
        let buffer = self.create_buffer(count as u64 * 72, memory)?;
        let id = self.res.rays.insert(buffer);
        self.res.table.rays.set(id, (count, memory));
        Ok(id)
    }

    pub fn destroy_ray_instances(&mut self, instances: u32) {
        self.res.table.rays.clear(instances);
        let buffer = self.res.rays.remove(instances);
        self.destroy_buffer(buffer);
    }

    pub fn ray_instances_handle(&self, instances: u32) -> Result<u64, String> {
        self.res.table.rays.get(instances);
        self.buffer_handle(*self.res.rays.get(instances))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_ray_instance(
        &self,
        instances: u32,
        index: u32,
        accel: u32,
        transform: &[f32; 12],
        id: u32,
        mask: u32,
        flags: u32,
    ) {
        self.res.table.ray_write(instances, index, id, mask, flags);
        let data = self
            .res
            .buffers
            .get(*self.res.rays.get(instances))
            .contents();
        let at = unsafe { data.as_ptr().cast::<u8>().add(index as usize * 72) };
        let out = unsafe { std::slice::from_raw_parts_mut(at, 72) };
        for (i, v) in transform.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out[48..52].copy_from_slice(&flags.to_le_bytes());
        out[52..56].copy_from_slice(&mask.to_le_bytes());
        out[56..60].copy_from_slice(&0u32.to_le_bytes());
        out[60..64].copy_from_slice(&id.to_le_bytes());
        out[64..72].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }
}
