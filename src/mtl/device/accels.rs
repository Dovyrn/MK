use super::{Device, id, vertex_format};
use crate::mtl::accel::Accel;
use crate::{AccelInfo, accel, index};
use objc2::rc::Retained;
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
            let d = MTL4InstanceAccelerationStructureDescriptor::new();
            unsafe {
                d.setInstanceDescriptorBuffer(self.range(
                    info.instances,
                    info.instance_offset,
                    info.instance_count as u64 * self.caps.instance as u64,
                ));
                d.setInstanceDescriptorStride(self.caps.instance as usize);
                d.setInstanceCount(info.instance_count as usize);
            }
            d.setInstanceDescriptorType(MTLAccelerationStructureInstanceDescriptorType::Indirect);
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
        self.res.table.accels.set(id, ());
        Ok(id)
    }

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
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
        self.res.table.accels.set(id, ());
        Ok(id)
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        self.res.table.accels.clear(accel);
        let a = self.res.accels.remove(accel);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*a.raw)));
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
        out[48..52].copy_from_slice(&flags.to_le_bytes());
        out[52..56].copy_from_slice(&mask.to_le_bytes());
        out[56..60].copy_from_slice(&0u32.to_le_bytes());
        out[60..64].copy_from_slice(&id.to_le_bytes());
        out[64..72].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }
}
