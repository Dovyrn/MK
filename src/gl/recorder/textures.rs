use super::Recorder;
use crate::command::Command;
use crate::gl::texture::TextureObject;
use crate::{Format, slot};
use glow::HasContext;

fn attachment_point(f: Format) -> u32 {
    match f.gl()[1] {
        glow::DEPTH_STENCIL => glow::DEPTH_STENCIL_ATTACHMENT,
        glow::DEPTH_COMPONENT => glow::DEPTH_ATTACHMENT,
        glow::STENCIL_INDEX => glow::STENCIL_ATTACHMENT,
        _ => glow::COLOR_ATTACHMENT0,
    }
}

fn clear_bits(point: u32) -> u32 {
    match point {
        glow::DEPTH_STENCIL_ATTACHMENT => glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT,
        glow::DEPTH_ATTACHMENT => glow::DEPTH_BUFFER_BIT,
        glow::STENCIL_ATTACHMENT => glow::STENCIL_BUFFER_BIT,
        _ => glow::COLOR_BUFFER_BIT,
    }
}

fn raw_name(t: &TextureObject) -> u32 {
    t.raw.map_or(0, |r| r.0.get())
}

impl Recorder<'_> {
    fn attach_level(&self, target: u32, t: &TextureObject, mip: u32, layer: u32) -> u32 {
        let gl = self.gl;
        let at = attachment_point(t.format);
        let mip = mip as i32;
        unsafe {
            match t.target {
                glow::TEXTURE_1D | glow::TEXTURE_2D | glow::TEXTURE_2D_MULTISAMPLE => {
                    gl.framebuffer_texture(target, at, t.raw, mip)
                }
                glow::TEXTURE_CUBE_MAP => {
                    let face = glow::TEXTURE_CUBE_MAP_POSITIVE_X + layer;
                    gl.framebuffer_texture_2d(target, at, face, t.raw, mip)
                }
                _ => gl.framebuffer_texture_layer(target, at, t.raw, mip, layer as i32),
            }
        }
        at
    }

    fn detach_all(&self, target: u32) {
        let gl = self.gl;
        unsafe {
            for i in 0..slot::COLORS {
                gl.framebuffer_texture(target, glow::COLOR_ATTACHMENT0 + i, None, 0);
            }
            gl.framebuffer_texture(target, glow::DEPTH_STENCIL_ATTACHMENT, None, 0);
        }
    }

    fn bind_copy_targets(
        &mut self,
        (src, src_mip, src_layer): (&TextureObject, u32, u32),
        (dst, dst_mip, dst_layer): (&TextureObject, u32, u32),
    ) -> u32 {
        let gl = self.gl;
        self.enable(glow::SCISSOR_TEST, false);
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.read));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.fbo));
            self.detach_all(glow::DRAW_FRAMEBUFFER);
            let at = self.attach_level(glow::READ_FRAMEBUFFER, src, src_mip, src_layer);
            self.attach_level(glow::DRAW_FRAMEBUFFER, dst, dst_mip, dst_layer);
            let color = at == glow::COLOR_ATTACHMENT0;
            gl.read_buffer(if color { at } else { glow::NONE });
            gl.draw_buffers(&[if color { at } else { glow::NONE }]);
            at
        }
    }

    fn unbind_copy_targets(&self, at: u32) {
        let gl = self.gl;
        unsafe {
            gl.framebuffer_texture(glow::READ_FRAMEBUFFER, at, None, 0);
            gl.framebuffer_texture(glow::DRAW_FRAMEBUFFER, at, None, 0);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn copy_texture_to_texture(&mut self, c: Command) {
        let Command::CopyTextureToTexture {
            src,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_z,
            dst,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_z,
            width,
            height,
            depth,
            layer_count: layers,
            ..
        } = c
        else {
            return;
        };
        let gl = self.gl;
        let res = self.res;
        let (s, d) = (res.textures.get(src), res.textures.get(dst));
        let line =
            |t: &TextureObject| matches!(t.target, glow::TEXTURE_1D | glow::TEXTURE_1D_ARRAY);
        if let Some(copy) = self.extra.copy_image {
            let at = |t: &TextureObject, layer: u32, x: u32, y: u32, z: u32| {
                match t.target {
                    _ if line(t) => [x, layer, 0],
                    glow::TEXTURE_3D => [x, y, z],
                    _ => [x, y, layer],
                }
                .map(|v| v as i32)
            };
            let [sx, sy, sz] = at(s, src_layer, src_x, src_y, src_z);
            let [dx, dy, dz] = at(d, dst_layer, dst_x, dst_y, dst_z);
            let [w, h, n] = if line(s) {
                [width, layers, 1]
            } else {
                [width, height, depth * layers]
            }
            .map(|v| v as i32);
            let (from, to) = (src_mip as i32, dst_mip as i32);
            unsafe {
                copy(
                    raw_name(s),
                    s.target,
                    from,
                    sx,
                    sy,
                    sz,
                    raw_name(d),
                    d.target,
                    to,
                    dx,
                    dy,
                    dz,
                    w,
                    h,
                    n,
                )
            };
            return;
        }
        let volume = |t: &TextureObject, layer: u32, z: u32| {
            if t.target == glow::TEXTURE_3D {
                z
            } else {
                layer
            }
        };
        let (from, to) = (volume(s, src_layer, src_z), volume(d, dst_layer, dst_z));
        let (x, y) = (src_x as i32, src_y as i32);
        let (u, v) = (dst_x as i32, dst_y as i32);
        let (w, h) = (width as i32, height as i32);
        let srgb = self.enabled.contains(&(glow::FRAMEBUFFER_SRGB, true));
        self.enable(glow::FRAMEBUFFER_SRGB, false);
        for i in 0..layers * depth {
            let at = self.bind_copy_targets((s, src_mip, from + i), (d, dst_mip, to + i));
            let bits = clear_bits(at);
            unsafe {
                gl.blit_framebuffer(x, y, x + w, y + h, u, v, u + w, v + h, bits, glow::NEAREST)
            };
            self.unbind_copy_targets(at);
        }
        self.enable(glow::FRAMEBUFFER_SRGB, srgb);
    }
}
