use super::Device;
use crate::gl::errors::{drain, raise};
use crate::gl::state::{BLOCKS, PIXELS, binding, buffer};
use crate::gl::texture::aspect;
use crate::{HostCopy, TextureCopy, format};
use glow::HasContext;
use std::ffi::c_void;

impl Device {
    pub fn write_texture(&mut self, copy: &HostCopy, data: &[u8]) -> Result<(), String> {
        self.thread();
        self.res.table.host(copy, data.len());
        let t = self.res.textures.get(copy.texture);
        let region = TextureCopy {
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
        self.around(t.target, true, |gl| unsafe {
            t.upload(gl, &self.extra, &region, data.as_ptr());
        })
    }

    pub fn read_texture(&mut self, copy: &HostCopy, data: &mut [u8]) -> Result<(), String> {
        self.thread();
        let (pitch, image) = self.res.table.host(copy, data.len());
        let info = *self.res.table.textures.get(copy.texture);
        let t = self.res.textures.get(copy.texture);
        let raw = t.raw.map_or(0, |r| r.0.get());
        let (f, ty) = aspect(t.format);
        let [bytes, bw, bh] = format::block(t.format);
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
        self.around(t.target, false, |gl| unsafe {
            match (e.sub_image, e.compressed_sub) {
                (Some(image), Some(compressed)) => {
                    gl.pixel_store_i32(glow::PACK_ROW_LENGTH, copy.row_length as i32);
                    gl.pixel_store_i32(glow::PACK_IMAGE_HEIGHT, copy.row_count as i32);
                    if f == 0 {
                        gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_WIDTH, bw as i32);
                        gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_HEIGHT, bh as i32);
                        gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_DEPTH, 1);
                        gl.pixel_store_i32(glow::PACK_COMPRESSED_BLOCK_SIZE, bytes as i32);
                        compressed(raw, mip, x, y, z, w, h, d, size, out);
                    } else {
                        image(raw, mip, x, y, z, w, h, d, f, ty, size, out);
                    }
                }
                _ => {
                    let level = |n: u32| (n >> copy.mip).max(1);
                    let (tall, images) = match t.target {
                        glow::TEXTURE_1D_ARRAY => (info.layers, 1),
                        glow::TEXTURE_3D => (level(info.height), level(info.depth)),
                        glow::TEXTURE_2D_ARRAY | glow::TEXTURE_CUBE_MAP_ARRAY => {
                            (level(info.height), info.layers)
                        }
                        _ => (level(info.height), 1),
                    };
                    let stride = level(info.width).div_ceil(bw) as usize * bytes as usize;
                    let slice = stride * tall.div_ceil(bh) as usize;
                    let mut all = vec![0u8; slice * images as usize];
                    let target = if t.target == glow::TEXTURE_CUBE_MAP {
                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + copy.layer
                    } else {
                        t.target
                    };
                    gl.bind_texture(t.target, t.raw);
                    let at = all.as_mut_ptr().cast();
                    if f == 0 {
                        (e.get_compressed)(target, mip, at);
                    } else {
                        (e.get_image)(target, mip, f, ty, at);
                    }
                    let first = (y as u32 / bh) as usize * stride;
                    let left = (copy.x / bw) as usize * bytes as usize;
                    let wide = copy.width.div_ceil(bw) as usize * bytes as usize;
                    for s in 0..copy.depth as usize {
                        let source = match t.target {
                            glow::TEXTURE_3D => copy.z as usize + s,
                            glow::TEXTURE_2D_ARRAY | glow::TEXTURE_CUBE_MAP_ARRAY => {
                                copy.layer as usize
                            }
                            _ => 0,
                        };
                        for r in 0..copy.height.div_ceil(bh) as usize {
                            let from = source * slice + first + r * stride + left;
                            let to = s * image as usize + r * pitch as usize;
                            data[to..to + wide].copy_from_slice(&all[from..from + wide]);
                        }
                    }
                }
            }
        })
    }

    fn around(
        &self,
        target: u32,
        unpack: bool,
        f: impl FnOnce(&glow::Context),
    ) -> Result<(), String> {
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
                Some(e) => Err(format!("host texture copy failed ({e:#x})")),
                None => Ok(()),
            }
        }
    }
}
