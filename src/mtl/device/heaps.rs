use super::{Device, options, texture_desc};
use crate::mtl::texture::Texture;
use crate::{TextureInfo, memory};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLHeap, MTLHeapDescriptor, MTLHeapType, MTLSparsePageSize};

impl Device {
    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        assert!(
            memory != memory::TRANSIENT,
            "transient memory is for textures"
        );
        let desc = MTLHeapDescriptor::new();
        desc.setType(MTLHeapType::Placement);
        desc.setResourceOptions(options(memory));
        desc.setSize(size as usize);
        desc.setMaxCompatiblePlacementSparsePageSize(MTLSparsePageSize::Size64);
        let heap = self
            .raw
            .newHeapWithDescriptor(&desc)
            .ok_or("heap allocation failed")?;
        self.residency.add(ProtocolObject::from_ref(&*heap));
        let id = self.res.heaps.insert((heap, memory));
        self.res.table.heaps.set(id, size);
        Ok(id)
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        let (heap, _) = self.res.heaps.remove(heap);
        self.defer(move |r| r.remove(ProtocolObject::from_ref(&*heap)));
    }

    pub fn buffer_size(&self, size: u64, memory: u32) -> (u64, u64) {
        let s = self
            .raw
            .heapBufferSizeAndAlignWithLength_options(size as usize, options(memory));
        (s.size as u64, s.align as u64)
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let s = self
            .raw
            .heapTextureSizeAndAlignWithDescriptor(&texture_desc(info));
        (s.size as u64, s.align as u64)
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        self.caps.buffer(size, memory::DEVICE)?;
        let (h, memory) = self.res.heaps.get(heap);
        let buffer = unsafe {
            h.newBufferWithLength_options_offset(size as usize, options(*memory), offset as usize)
                .ok_or("placed buffer allocation failed")?
        };
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        self.supports(info)?;
        let (h, memory) = self.res.heaps.get(heap);
        let desc = texture_desc(info);
        desc.setResourceOptions(options(*memory));
        let raw = unsafe {
            h.newTextureWithDescriptor_offset(&desc, offset as usize)
                .ok_or("placed texture allocation failed")?
        };
        let id = self.res.textures.insert(Texture {
            raw,
            format: info.format,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }
}
