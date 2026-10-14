use super::Device;
use crate::Error;
use crate::gl::errors::{drain, raise};
use crate::gl::state::{BLOCKS, PIXELS, binding, buffer};
use crate::gl::texture::aspect;
use crate::{BufferTextureCopy, HostTextureCopy};
use glow::HasContext;
use std::ffi::c_void;

impl Device {
    pub fn write_texture(&self, copy: &HostTextureCopy, data: &[u8]) -> Result<(), Error> {
        self.thread();
        self.res.table.check_host_copy(copy, data.len());
        let t = self.res.textures.get(copy.texture);
        let region = BufferTextureCopy {
            row_length: copy.row_length,
            row_count: copy.row_count,
            texture: copy.texture,
            mip: copy.mip,
            layer: copy.layer,
            x: copy.x,
            y: copy.y,
            z: copy.z,
            width: copy.width,
            height: copy.height,
            depth: copy.depth,
            ..Default::default()
        };
        self.with_pixel_state(t.target, true, |gl| unsafe {
            t.upload(gl, &self.extra, &region, data.as_ptr());
        })
    }

    pub fn read_texture(&self, copy: &HostTextureCopy, data: &mut [u8]) -> Result<(), Error> {
        self.thread();
        self.res.table.check_host_copy(copy, data.len());
        if !self.caps.host_read {
            return Err(Error::Unsupported(
                "read_texture needs caps.host_read".into(),
            ));
        }
        let t = self.res.textures.get(copy.texture);
        let raw = t.raw.map_or(0, |r| r.0.get());
        let (f, ty) = aspect(t.format);
        let [bytes, bw, bh] = t.format.block();
        let (x, w, mip) = (copy.x as i32, copy.width as i32, copy.mip as i32);
        let (y, h, z, d) = match t.target {
            glow::TEXTURE_1D_ARRAY => (copy.layer, 1, 0, 1),
            glow::TEXTURE_1D | glow::TEXTURE_2D => (copy.y, copy.height, 0, 1),
            glow::TEXTURE_3D => (copy.y, copy.height, copy.z, copy.depth),
            _ => (copy.y, copy.height, copy.layer, 1),
        };
        let (y, h, z, d) = (y as i32, h as i32, z as i32, d as i32);
        let size = data.len() as i32;
        let out = data.as_mut_ptr().cast::<c_void>();
        let e = &self.extra;
        self.with_pixel_state(t.target, false, |gl| unsafe {
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, copy.row_length as i32);
            gl.pixel_store_i32(glow::PACK_IMAGE_HEIGHT, copy.row_count as i32);
            if f == 0 {
                gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_WIDTH, bw as i32);
                gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_HEIGHT, bh as i32);
                gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_DEPTH, 1);
                gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_SIZE, bytes as i32);
                e.compressed_sub.unwrap()(raw, mip, x, y, z, w, h, d, size, out);
            } else {
                e.sub_image.unwrap()(raw, mip, x, y, z, w, h, d, f, ty, size, out);
            }
        })
    }

    fn with_pixel_state(
        &self,
        target: u32,
        unpack: bool,
        f: impl FnOnce(&glow::Context),
    ) -> Result<(), Error> {
        let gl = &self.gl;
        let (point, bound, half) = if unpack {
            (
                glow::PIXEL_UNPACK_BUFFER,
                glow::PIXEL_UNPACK_BUFFER_BINDING,
                0,
            )
        } else {
            (glow::PIXEL_PACK_BUFFER, glow::PIXEL_PACK_BUFFER_BINDING, 1)
        };
        let mut names = PIXELS[half * 8..half * 8 + 8].to_vec();
        if self.has(42, "GL_ARB_compressed_texture_pixel_storage") {
            names.extend(&BLOCKS[half * 4..half * 4 + 4]);
        }
        unsafe {
            let host = drain(gl);
            let int = |p| gl.get_parameter_i32(p);
            let unit = int(glow::ACTIVE_TEXTURE) as u32;
            gl.active_texture(glow::TEXTURE0);
            let old = int(binding(target)) as u32;
            let buffer = buffer(int(bound));
            let pixels: Vec<(u32, i32)> = names.iter().map(|&p| (p, int(p))).collect();
            for &p in &names {
                let aligned = p == glow::UNPACK_ALIGNMENT || p == glow::PACK_ALIGNMENT;
                gl.pixel_store_i32(p, aligned as i32);
            }
            gl.bind_buffer(point, None);
            f(gl);
            for (p, v) in pixels {
                gl.pixel_store_i32(p, v);
            }
            gl.bind_buffer(point, buffer);
            let old = std::num::NonZeroU32::new(old).map(glow::NativeTexture);
            gl.bind_texture(target, old);
            gl.active_texture(unit);
            let errors = drain(gl);
            raise(gl, &self.extra, &host);
            match errors.first() {
                Some(e) => Err(Error::Driver(format!("host texture copy failed ({e:#x})"))),
                None => Ok(()),
            }
        }
    }
}
