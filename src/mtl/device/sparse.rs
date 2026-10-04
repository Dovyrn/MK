use super::{Device, kind, options, size, texture_desc};
use crate::mtl::texture::Texture;
use crate::{SparseRange, SparseRegion, TextureInfo, format, memory};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_foundation::NSRange;
use objc2_metal::{
    MTL4CommandQueue, MTL4UpdateSparseBufferMappingOperation,
    MTL4UpdateSparseTextureMappingOperation, MTLDevice, MTLHeap, MTLOrigin, MTLRegion,
    MTLSparsePageSize, MTLSparseTextureMappingMode,
};
use std::ptr::NonNull;

impl Device {
    fn sparse_check(&self) -> Result<(), String> {
        if self.caps.sparse {
            Ok(())
        } else {
            Err("sparse residency is not supported".into())
        }
    }

    pub fn sparse_page(&self) -> Result<u64, String> {
        self.sparse_check()?;
        Ok(self
            .raw
            .sparseTileSizeInBytesForSparsePageSize(MTLSparsePageSize::Size64) as u64)
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        self.sparse_check()?;
        self.supports(info)?;
        let s = unsafe {
            self.raw
                .sparseTileSizeWithTextureType_pixelFormat_sampleCount_sparsePageSize(
                    kind(info.dimension, info.samples),
                    format::mtl(info.format),
                    info.samples as usize,
                    MTLSparsePageSize::Size64,
                )
        };
        if s.width == 0 {
            return Err(format!("format {} has no sparse layout", info.format));
        }
        Ok([s.width as u32, s.height as u32, s.depth as u32])
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        autoreleasepool(|_| {
            self.caps.buffer(size, memory::DEVICE)?;
            self.sparse_check()?;
            let buffer = unsafe {
                self.raw
                    .newBufferWithLength_options_placementSparsePageSize(
                        size as usize,
                        options(memory::DEVICE),
                        MTLSparsePageSize::Size64,
                    )
                    .ok_or("sparse buffer allocation failed")?
            };
            self.residency.add(ProtocolObject::from_ref(&*buffer));
            let id = self.res.buffers.insert(buffer);
            self.res.table.buffers.set(id, size);
            self.res.table.pages.set(id, ());
            Ok(id)
        })
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        autoreleasepool(|_| {
            let tile = self.sparse_tile(info)?;
            let desc = texture_desc(info);
            desc.setPlacementSparsePageSize(MTLSparsePageSize::Size64);
            let raw = self
                .raw
                .newTextureWithDescriptor(&desc)
                .ok_or("sparse texture allocation failed")?;
            self.residency.add(ProtocolObject::from_ref(&*raw));
            let id = self.res.textures.insert(Texture {
                raw,
                format: info.format,
            });
            self.res.table.textures.set(id, *info);
            self.res.table.tiles.set(id, tile);
            Ok(id)
        })
    }

    fn sparse(
        &self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        f: impl FnOnce(&ProtocolObject<dyn MTL4CommandQueue>, Option<&ProtocolObject<dyn MTLHeap>>),
        heap: u32,
    ) -> Result<(), String> {
        self.sparse_check()?;
        let q = &self.queues[queue].raw;
        for &(event, value) in waits {
            q.waitForEvent_value(
                ProtocolObject::from_ref(&**self.res.events.get(event)),
                value,
            );
        }
        let heap = (heap != !0).then(|| &*self.res.heaps.get(heap).0);
        f(q, heap);
        for &(event, value) in signals {
            q.signalEvent_value(
                ProtocolObject::from_ref(&**self.res.events.get(event)),
                value,
            );
        }
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
        autoreleasepool(|_| {
            self.check_queue(queue);
            self.res.table.buffers.get(buffer);
            let page = self.sparse_page()?;
            self.res.table.map_buffer(buffer, heap, page, ranges);
            self.res.table.submit(queue, waits, signals);
            let mode = if heap == !0 {
                MTLSparseTextureMappingMode::Unmap
            } else {
                MTLSparseTextureMappingMode::Map
            };
            let mut ops: Vec<_> = ranges
                .iter()
                .map(|r| MTL4UpdateSparseBufferMappingOperation {
                    mode,
                    bufferRange: NSRange::new(r.page as usize, r.count as usize),
                    heapOffset: r.heap_page as usize,
                })
                .collect();
            let raw = self.res.buffers.get(buffer);
            self.sparse(
                queue,
                waits,
                signals,
                |q, h| unsafe {
                    q.updateBufferMappings_heap_operations_count(
                        raw,
                        h,
                        NonNull::new(ops.as_mut_ptr()).unwrap(),
                        ops.len(),
                    )
                },
                heap,
            )?;
            self.res.table.signal(queue, signals);
            self.queues[queue].signal();
            Ok(())
        })
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
        autoreleasepool(|_| {
            self.check_queue(queue);
            self.res.table.textures.get(texture);
            let page = self.sparse_page()?;
            self.res.table.map_texture(texture, heap, page, regions);
            self.res.table.submit(queue, waits, signals);
            let mode = if heap == !0 {
                MTLSparseTextureMappingMode::Unmap
            } else {
                MTLSparseTextureMappingMode::Map
            };
            let mut ops: Vec<_> = regions
                .iter()
                .map(|r| MTL4UpdateSparseTextureMappingOperation {
                    mode,
                    textureRegion: MTLRegion {
                        origin: MTLOrigin {
                            x: r.x as usize,
                            y: r.y as usize,
                            z: r.z as usize,
                        },
                        size: size([r.width, r.height, r.depth]),
                    },
                    textureLevel: r.mip as usize,
                    textureSlice: r.layer as usize,
                    heapOffset: r.heap_page as usize,
                })
                .collect();
            let raw = &self.res.textures.get(texture).raw;
            self.sparse(
                queue,
                waits,
                signals,
                |q, h| unsafe {
                    q.updateTextureMappings_heap_operations_count(
                        raw,
                        h,
                        NonNull::new(ops.as_mut_ptr()).unwrap(),
                        ops.len(),
                    )
                },
                heap,
            )?;
            self.res.table.signal(queue, signals);
            self.queues[queue].signal();
            Ok(())
        })
    }
}
