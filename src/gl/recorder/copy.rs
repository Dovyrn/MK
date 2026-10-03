use super::Recorder;
use crate::gl::texture::aspect;
use crate::{Command, TextureCopy, format};
use glow::HasContext;

impl Recorder<'_> {
    pub fn upload(&self, c: Command) {
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
        let copy = TextureCopy {
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
            ..Default::default()
        };
        let raw = self.res.buffers.get(buffer).raw;
        unsafe {
            self.gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(raw));
            let t = self.res.textures.get(texture);
            t.upload(self.gl, self.extra, &copy, offset as usize as *const u8);
            self.gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
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
            let raw = res.buffers.get(buffer).raw;
            if f == 0 {
                let read = self
                    .extra
                    .compressed_sub
                    .ok_or("compressed readback needs GL_ARB_get_texture_sub_image")?;
                gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(raw));
                let [bytes, bw, bh] = format::block(t.format);
                let store = [
                    (glow::PACK_ROW_LENGTH, row_length),
                    (glow::PACK_IMAGE_HEIGHT, row_count),
                    (glow::PACK_COMPRESSED_BLOCK_WIDTH, bw),
                    (glow::PACK_COMPRESSED_BLOCK_HEIGHT, bh),
                    (glow::PACK_COMPRESSED_BLOCK_DEPTH, 1),
                    (glow::PACK_COMPRESSED_BLOCK_SIZE, bytes),
                ];
                for (p, v) in store {
                    gl.pixel_store_i32(p, v as i32);
                }
                let (z, depth) = match t.target {
                    glow::TEXTURE_3D => (z, depth),
                    glow::TEXTURE_2D => (0, 1),
                    _ => (layer, 1),
                };
                let size = (res.buffers.get(buffer).size - offset).min(i32::MAX as u64);
                read(
                    t.raw.map_or(0, |r| r.0.get()),
                    mip as i32,
                    x as i32,
                    y as i32,
                    z as i32,
                    width as i32,
                    height as i32,
                    depth as i32,
                    size as i32,
                    offset as usize as *mut std::ffi::c_void,
                );
                for (p, _) in store {
                    gl.pixel_store_i32(p, 0);
                }
                gl.bind_buffer(glow::PIXEL_PACK_BUFFER, None);
                return Ok(());
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
            gl.bind_buffer(glow::PIXEL_PACK_BUFFER, Some(raw));
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
