use super::Device;
use crate::gl::buffer::Buffer;
use crate::gl::errors::{drain, raise};
use crate::gl::sparse::PAGE;
use crate::gl::state::{self, binding};
use crate::gl::texture::{Texture, target};
use crate::gl::tiles::{SIZE, SIZES};
use crate::{SparseRange, SparseRegion, TextureInfo, dimension, format, memory};
use glow::HasContext;

impl Device {
    pub fn sparse_page(&self) -> Result<u64, String> {
        self.thread();
        if !self.caps.sparse {
            return Err(
                "sparse residency needs GL_ARB_sparse_buffer and GL_NV_memory_object_sparse".into(),
            );
        }
        Ok(unsafe { self.gl.get_parameter_i32(PAGE) } as u64)
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        self.thread();
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)?;
        if !self.caps.sparse_textures {
            return Err(
                "sparse textures need GL_ARB_sparse_texture and GL_NV_memory_object_sparse".into(),
            );
        }
        let kind = target(info.dimension, info.samples);
        let internal = format::gl(info.format)[0];
        let query = |p| {
            let mut v = [0];
            unsafe {
                self.gl
                    .get_internal_format_i32_slice(kind, internal, p, &mut v)
            };
            v[0]
        };
        if query(SIZES) == 0 {
            return Err(format!("format {} has no sparse layout", info.format));
        }
        Ok(SIZE.map(|p| query(p) as u32))
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        self.thread();
        self.caps.buffer(size, memory::DEVICE)?;
        self.sparse_page()?;
        let buffer = Buffer::sparse(&self.gl, &self.extra, size)?;
        let id = self.res.buffers.insert(buffer);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.sparse_tile(info)?;
        let texture = Texture::new(&self.gl, &self.extra, info, true, None, true)?;
        let id = self.res.textures.insert(texture);
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    fn memory(&self, heap: u32) -> Option<u32> {
        (heap != !0).then(|| {
            self.res.table.heaps.get(heap);
            *self.res.heaps.get(heap)
        })
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
        assert_eq!(queue, 0, "GL has one queue");
        self.res.table.buffers.get(buffer);
        let page = self.sparse_page()?;
        let memory = self.memory(heap);
        self.res.table.submit(queue, waits, signals);
        self.healthy()?;
        self.waits(waits);
        let s = self.extra.sparse.as_ref().unwrap();
        let gl = &self.gl;
        let target = glow::COPY_WRITE_BUFFER;
        let errors = unsafe {
            let host = drain(gl);
            let old = state::buffer(gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            gl.bind_buffer(target, Some(self.res.buffers.get(buffer).raw));
            for r in ranges {
                let at = (r.page as u64 * page) as isize;
                let size = (r.count as u64 * page) as isize;
                match memory {
                    Some(m) => (s.commit)(target, at, size, m, r.heap_page as u64 * page, 1),
                    None => (s.release)(target, at, size, 0),
                }
            }
            gl.bind_buffer(target, old);
            let errors = drain(gl);
            raise(gl, &self.extra, &host);
            errors
        };
        if let Some(e) = errors.first() {
            return Err(format!("sparse buffer mapping failed ({e:#x})"));
        }
        self.signals(queue, signals)
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
        assert_eq!(queue, 0, "GL has one queue");
        let info = *self.res.table.textures.get(texture);
        let [tw, th, td] = self.sparse_tile(&info)?;
        let page = self.sparse_page()?;
        let memory = self.memory(heap);
        self.res.table.submit(queue, waits, signals);
        self.healthy()?;
        self.waits(waits);
        let tiles = self.extra.tiles.as_ref().unwrap();
        let t = self.res.textures.get(texture);
        let volume = info.dimension == dimension::D3;
        let gl = &self.gl;
        let errors = unsafe {
            let host = drain(gl);
            let unit = gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
            gl.active_texture(glow::TEXTURE0);
            let old = gl.get_parameter_i32(binding(t.target)) as u32;
            gl.bind_texture(t.target, t.raw);
            for r in regions {
                let edge = |n: u32| (n >> r.mip).max(1);
                let (x, y) = (r.x * tw, r.y * th);
                let w = (r.width * tw).min(edge(info.width) - x);
                let h = (r.height * th).min(edge(info.height) - y);
                let (layer, z, d) = if volume {
                    let z = r.z * td;
                    (0, z, (r.depth * td).min(edge(info.depth) - z))
                } else {
                    (r.layer, 0, 1)
                };
                let [mip, x, y, z, w, h, d] = [r.mip, x, y, z, w, h, d].map(|v| v as i32);
                match memory {
                    Some(m) => {
                        let offset = r.heap_page as u64 * page;
                        let commit = tiles.commit;
                        let layer = layer as i32;
                        commit(t.target, layer, mip, x, y, z, w, h, d, m, offset, 1)
                    }
                    None if volume => (tiles.release)(t.target, mip, x, y, z, w, h, d, 0),
                    None => (tiles.release)(t.target, mip, x, y, layer as i32, w, h, 1, 0),
                }
            }
            let old = std::num::NonZeroU32::new(old).map(glow::NativeTexture);
            gl.bind_texture(t.target, old);
            gl.active_texture(unit);
            let errors = drain(gl);
            raise(gl, &self.extra, &host);
            errors
        };
        if let Some(e) = errors.first() {
            return Err(format!("sparse texture mapping failed ({e:#x})"));
        }
        self.signals(queue, signals)
    }
}
