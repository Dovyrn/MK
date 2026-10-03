use super::Device;
use crate::gl::buffer::Buffer;
use crate::gl::errors::{drain, raise};
use crate::gl::memory::OPAQUE_FD;
use crate::gl::texture::Texture;
use crate::{TextureInfo, memory};

impl Device {
    pub fn import_heap(&mut self, fd: i32, size: u64, memory: u32) -> Result<u32, String> {
        self.thread();
        assert_ne!(
            memory,
            memory::TRANSIENT,
            "transient memory is for textures"
        );
        let m = self
            .extra
            .memory
            .as_ref()
            .ok_or("shared heaps need GL_EXT_memory_object_fd")?;
        if memory != memory::DEVICE {
            return Err(
                "GL memory objects cannot be mapped; import the heap as device memory".into(),
            );
        }
        let gl = &self.gl;
        unsafe {
            let host = drain(gl);
            let mut raw = 0;
            (m.create)(1, &mut raw);
            (m.import)(raw, size, OPAQUE_FD, fd);
            let error = drain(gl).first().copied();
            if error.is_some() {
                (m.delete)(1, &raw);
            }
            raise(gl, &self.extra, &host);
            if let Some(e) = error {
                return Err(format!("importing fd {fd} failed ({e:#x})"));
            }
            let id = self.res.heaps.insert(raw);
            self.res.table.heaps.set(id, size);
            Ok(id)
        }
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        self.thread();
        self.res.table.heaps.clear(heap);
        let raw = self.res.heaps.remove(heap);
        let m = self.extra.memory.as_ref().unwrap();
        unsafe { (m.delete)(1, &raw) };
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        self.thread();
        self.res.table.heaps.get(heap);
        self.caps.buffer(size, memory::DEVICE)?;
        let raw = *self.res.heaps.get(heap);
        let buffer = Buffer::placed(&self.gl, &self.extra, (raw, offset), size)?;
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
        self.thread();
        self.res.table.heaps.get(heap);
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)?;
        let place = Some((*self.res.heaps.get(heap), offset));
        let texture = Texture::new(&self.gl, &self.extra, info, true, place)?;
        let id = self.res.textures.insert(texture);
        self.res.table.textures.set(id, *info);
        Ok(id)
    }
}
