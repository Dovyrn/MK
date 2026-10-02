use super::Recorder;
use crate::{Command, format};
use glow::HasContext;

fn aspect(f: u32) -> (u32, u32) {
    match f {
        format::D24_UNORM_S8 => (glow::DEPTH_COMPONENT, glow::UNSIGNED_INT),
        format::D32_FLOAT_S8 => (glow::DEPTH_COMPONENT, glow::FLOAT),
        f => {
            let [_, f, ty] = format::gl(f);
            (f, ty)
        }
    }
}

impl Recorder<'_> {
    pub fn upload(&self, c: Command) {
        let gl = self.gl;
        let res = self.res;
        let Command::CopyBufferToTexture {
            buffer,
            offset,
            row_length,
            row_count,
            texture,
            mip,
            layer,
            x,
            y,
            z,
            width,
            height,
            depth,
            ..
        } = c
        else {
            return;
        };
        unsafe {
            let t = res.textures.get(texture);
            let internal = format::gl(t.format)[0];
            let (f, ty) = aspect(t.format);
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(res.buffers.get(buffer).raw));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, row_length as i32);
            gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, row_count as i32);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(t.target, t.raw);
            let data = offset as usize as *const std::ffi::c_void;
            let (w, h, d) = (width as i32, height as i32, depth as i32);
            let (mip, x, y) = (mip as i32, x as i32, y as i32);
            let e = self.extra;
            if f == 0 {
                let [bytes, bw, bh] = format::block(t.format);
                let wide = width.div_ceil(bw);
                let pitch = row_length.max(width).div_ceil(bw) as usize * bytes as usize;
                let rows = row_count.max(height).div_ceil(bh) as usize;
                let tall = height.div_ceil(bh) as usize;
                let step = if pitch == wide as usize * bytes as usize {
                    tall
                } else {
                    1
                };
                for s in 0..depth as usize {
                    for r in (0..tall).step_by(step) {
                        let n = step.min(tall - r);
                        let top = y + (r as u32 * bh) as i32;
                        let h = (n as i32 * bh as i32).min(h - (r as u32 * bh) as i32);
                        let at = offset as usize + s * rows * pitch + r * pitch;
                        let at = at as *const std::ffi::c_void;
                        let size = (n * wide as usize * bytes as usize) as i32;
                        match t.target {
                            glow::TEXTURE_2D | glow::TEXTURE_CUBE_MAP => {
                                let target = if t.target == glow::TEXTURE_2D {
                                    t.target
                                } else {
                                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer
                                };
                                (e.compressed2)(target, mip, x, top, w, h, internal, size, at)
                            }
                            _ => {
                                let z = if t.target == glow::TEXTURE_3D {
                                    (z as usize + s) as i32
                                } else {
                                    layer as i32
                                };
                                (e.compressed3)(
                                    t.target, mip, x, top, z, w, h, 1, internal, size, at,
                                )
                            }
                        }
                    }
                }
            } else {
                match t.target {
                    glow::TEXTURE_1D => (e.image1)(t.target, mip, x, w, f, ty, data),
                    glow::TEXTURE_2D => (e.image2)(t.target, mip, x, y, w, h, f, ty, data),
                    glow::TEXTURE_1D_ARRAY => {
                        (e.image2)(t.target, mip, x, layer as i32, w, 1, f, ty, data)
                    }
                    glow::TEXTURE_CUBE_MAP => (e.image2)(
                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer,
                        mip,
                        x,
                        y,
                        w,
                        h,
                        f,
                        ty,
                        data,
                    ),
                    glow::TEXTURE_3D => {
                        (e.image3)(t.target, mip, x, y, z as i32, w, h, d, f, ty, data)
                    }
                    _ => (e.image3)(t.target, mip, x, y, layer as i32, w, h, 1, f, ty, data),
                }
            }
            gl.bind_texture(t.target, None);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, 0);
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
        }
    }

    pub fn readback(&self, c: Command) -> Result<(), String> {
        let gl = self.gl;
        let res = self.res;
        let Command::CopyTextureToBuffer {
            texture,
            mip,
            layer,
            x,
            y,
            z,
            width,
            height,
            depth,
            buffer,
            offset,
            row_length,
            row_count,
            ..
        } = c
        else {
            return Ok(());
        };
        unsafe {
            let t = res.textures.get(texture);
            let (f, ty) = aspect(t.format);
            if f == 0 {
                return Err("compressed readback is not supported on GL".into());
            }
            let point = match f {
                glow::DEPTH_COMPONENT => glow::DEPTH_ATTACHMENT,
                glow::STENCIL_INDEX => glow::STENCIL_ATTACHMENT,
                _ => glow::COLOR_ATTACHMENT0,
            };
            let target = glow::READ_FRAMEBUFFER;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.read));
            gl.draw_buffers(&[glow::NONE]);
            gl.read_buffer(if point == glow::COLOR_ATTACHMENT0 {
                point
            } else {
                glow::NONE
            });
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(res.buffers.get(buffer).raw));
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, row_length as i32);
            let texel = match f {
                glow::DEPTH_COMPONENT => 4.min(format::block(t.format)[0]),
                _ => format::block(t.format)[0],
            } as u64;
            let slice = (row_length.max(width) * row_count.max(height)) as u64 * texel;
            for s in 0..depth {
                let mip = mip as i32;
                match t.target {
                    glow::TEXTURE_1D | glow::TEXTURE_2D | glow::TEXTURE_2D_MULTISAMPLE => {
                        gl.framebuffer_texture(target, point, t.raw, mip)
                    }
                    glow::TEXTURE_CUBE_MAP => gl.framebuffer_texture_2d(
                        target,
                        point,
                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer,
                        t.raw,
                        mip,
                    ),
                    glow::TEXTURE_3D => {
                        let z = (z + s) as i32;
                        gl.framebuffer_texture_layer(target, point, t.raw, mip, z)
                    }
                    _ => gl.framebuffer_texture_layer(target, point, t.raw, mip, layer as i32),
                }
                (self.extra.read)(
                    x as i32,
                    y as i32,
                    width as i32,
                    height as i32,
                    f,
                    ty,
                    (offset + s as u64 * slice) as usize as *mut std::ffi::c_void,
                );
            }
            gl.pixel_store_i32(glow::PACK_ROW_LENGTH, 0);
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
            gl.framebuffer_texture(target, point, None, 0);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        Ok(())
    }
}
