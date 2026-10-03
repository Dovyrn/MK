use super::Recorder;
use crate::gl::texture::Texture;
use crate::{Command, filter, format, slot};
use glow::HasContext;

fn point(f: u32) -> u32 {
    match format::gl(f)[1] {
        glow::DEPTH_STENCIL => glow::DEPTH_STENCIL_ATTACHMENT,
        glow::DEPTH_COMPONENT => glow::DEPTH_ATTACHMENT,
        glow::STENCIL_INDEX => glow::STENCIL_ATTACHMENT,
        _ => glow::COLOR_ATTACHMENT0,
    }
}

fn mask(point: u32) -> u32 {
    match point {
        glow::DEPTH_STENCIL_ATTACHMENT => glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT,
        glow::DEPTH_ATTACHMENT => glow::DEPTH_BUFFER_BIT,
        glow::STENCIL_ATTACHMENT => glow::STENCIL_BUFFER_BIT,
        _ => glow::COLOR_BUFFER_BIT,
    }
}

fn srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

fn bytes<T: Copy, const N: usize>(values: [T; N]) -> Vec<u8> {
    let size = std::mem::size_of_val(&values);
    unsafe { std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), size) }.to_vec()
}

fn name(t: &Texture) -> u32 {
    t.raw.map_or(0, |r| r.0.get())
}

fn rest(n: u32, base: u32, total: u32) -> u32 {
    if n == !0 { total - base } else { n }
}

impl Recorder<'_> {
    fn level(&self, target: u32, t: &Texture, mip: u32, layer: u32) -> u32 {
        let gl = self.gl;
        let at = point(t.format);
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

    fn clean(&self, target: u32) {
        let gl = self.gl;
        unsafe {
            for i in 0..slot::COLORS {
                gl.framebuffer_texture(target, glow::COLOR_ATTACHMENT0 + i, None, 0);
            }
            gl.framebuffer_texture(target, glow::DEPTH_STENCIL_ATTACHMENT, None, 0);
        }
    }

    fn pair(
        &self,
        (src, src_mip, src_layer): (&Texture, u32, u32),
        (dst, dst_mip, dst_layer): (&Texture, u32, u32),
    ) -> u32 {
        let gl = self.gl;
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.read));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.fbo));
            self.clean(glow::DRAW_FRAMEBUFFER);
            let at = self.level(glow::READ_FRAMEBUFFER, src, src_mip, src_layer);
            self.level(glow::DRAW_FRAMEBUFFER, dst, dst_mip, dst_layer);
            let color = at == glow::COLOR_ATTACHMENT0;
            gl.read_buffer(if color { at } else { glow::NONE });
            gl.draw_buffers(&[if color { at } else { glow::NONE }]);
            at
        }
    }

    fn unpair(&self, at: u32) {
        let gl = self.gl;
        unsafe {
            gl.framebuffer_texture(glow::READ_FRAMEBUFFER, at, None, 0);
            gl.framebuffer_texture(glow::DRAW_FRAMEBUFFER, at, None, 0);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
    }

    pub fn copy_texture(&self, c: Command) {
        let Command::CopyTexture {
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
            layers,
            ..
        } = c
        else {
            return;
        };
        let gl = self.gl;
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let line = |t: &Texture| matches!(t.target, glow::TEXTURE_1D | glow::TEXTURE_1D_ARRAY);
        if let Some(copy) = self.extra.copy_image {
            let at = |t: &Texture, layer: u32, x: u32, y: u32, z: u32| {
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
                    name(s),
                    s.target,
                    from,
                    sx,
                    sy,
                    sz,
                    name(d),
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
        let volume = |t: &Texture, layer: u32, z: u32| {
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
        unsafe {
            gl.disable(glow::FRAMEBUFFER_SRGB);
            for i in 0..layers * depth {
                let at = self.pair((s, src_mip, from + i), (d, dst_mip, to + i));
                let bits = mask(at);
                gl.blit_framebuffer(x, y, x + w, y + h, u, v, u + w, v + h, bits, glow::NEAREST);
                self.unpair(at);
            }
            gl.enable(glow::FRAMEBUFFER_SRGB);
        }
    }

    pub fn blit_texture(&self, c: Command) {
        let Command::BlitTexture {
            src,
            src_mip,
            src_layer,
            src_x,
            src_y,
            src_width,
            src_height,
            dst,
            dst_mip,
            dst_layer,
            dst_x,
            dst_y,
            dst_width,
            dst_height,
            filter: f,
            ..
        } = c
        else {
            return;
        };
        let (s, d) = (self.res.textures.get(src), self.res.textures.get(dst));
        let at = self.pair((s, src_mip, src_layer), (d, dst_mip, dst_layer));
        let (x, y) = (src_x as i32, src_y as i32);
        let (w, h) = (src_width as i32, src_height as i32);
        let (u, v) = (dst_x as i32, dst_y as i32);
        let (p, q) = (dst_width as i32, dst_height as i32);
        let filter = if f == filter::NEAREST {
            glow::NEAREST
        } else {
            glow::LINEAR
        };
        unsafe {
            self.gl
                .blit_framebuffer(x, y, x + w, y + h, u, v, u + p, v + q, mask(at), filter)
        };
        self.unpair(at);
    }

    pub fn clear_texture(&self, c: Command) {
        let Command::ClearTexture {
            texture,
            base_mip,
            mips,
            base_layer,
            layers,
            r,
            g,
            b,
            a,
            depth,
            stencil,
            ..
        } = c
        else {
            return;
        };
        let gl = self.gl;
        let t = self.res.textures.get(texture);
        let info = *self.res.table.textures.get(texture);
        let mips = rest(mips, base_mip, info.mips);
        let layers = rest(layers, base_layer, info.layers);
        let at = point(t.format);
        let color = [r, g, b, a];
        let encoded = if matches!(t.format, format::RGBA8_SRGB | format::BGRA8_SRGB) {
            [srgb(r), srgb(g), srgb(b), a]
        } else {
            color
        };
        let sizes = |m: u32| [info.width, info.height, info.depth].map(|n| (n >> m).max(1));
        if let Some(clear) = self.extra.clear_image {
            let (f, ty, data) = match at {
                glow::COLOR_ATTACHMENT0 if format::uint(t.format) => (
                    glow::RGBA_INTEGER,
                    glow::UNSIGNED_INT,
                    bytes(color.map(|v| v as u32)),
                ),
                glow::COLOR_ATTACHMENT0 if format::sint(t.format) => (
                    glow::RGBA_INTEGER,
                    glow::INT,
                    bytes(color.map(|v| v as i32)),
                ),
                glow::COLOR_ATTACHMENT0 => (glow::RGBA, glow::FLOAT, bytes(encoded)),
                glow::DEPTH_ATTACHMENT => (glow::DEPTH_COMPONENT, glow::FLOAT, bytes([depth])),
                glow::DEPTH_STENCIL_ATTACHMENT => {
                    let mut data = bytes([depth]);
                    data.extend(bytes([stencil]));
                    let ty = glow::FLOAT_32_UNSIGNED_INT_24_8_REV;
                    (glow::DEPTH_STENCIL, ty, data)
                }
                _ => (
                    glow::STENCIL_INDEX,
                    glow::UNSIGNED_BYTE,
                    vec![stencil as u8],
                ),
            };
            for m in base_mip..base_mip + mips {
                let [w, h, d] = sizes(m);
                let [y, z, tall, deep] = match t.target {
                    glow::TEXTURE_1D_ARRAY => [base_layer, 0, layers, 1],
                    glow::TEXTURE_3D => [0, 0, h, d],
                    _ => [0, base_layer, h, layers],
                }
                .map(|v| v as i32);
                let ptr = data.as_ptr().cast();
                unsafe { clear(name(t), m as i32, 0, y, z, w as i32, tall, deep, f, ty, ptr) };
            }
            return;
        }
        let target = glow::FRAMEBUFFER;
        unsafe {
            gl.bind_framebuffer(target, Some(self.read));
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::FRAMEBUFFER_SRGB);
            gl.color_mask_draw_buffer(0, true, true, true, true);
            gl.depth_mask(true);
            gl.stencil_mask(0xff);
            let colored = at == glow::COLOR_ATTACHMENT0;
            gl.draw_buffers(&[if colored { at } else { glow::NONE }]);
            for m in base_mip..base_mip + mips {
                let (first, count) = if t.target == glow::TEXTURE_3D {
                    (0, sizes(m)[2])
                } else {
                    (base_layer, layers)
                };
                for l in first..first + count {
                    self.level(target, t, m, l);
                    match at {
                        glow::COLOR_ATTACHMENT0 if format::uint(t.format) => {
                            gl.clear_buffer_u32_slice(glow::COLOR, 0, &color.map(|v| v as u32))
                        }
                        glow::COLOR_ATTACHMENT0 if format::sint(t.format) => {
                            gl.clear_buffer_i32_slice(glow::COLOR, 0, &color.map(|v| v as i32))
                        }
                        glow::COLOR_ATTACHMENT0 => {
                            gl.clear_buffer_f32_slice(glow::COLOR, 0, &encoded)
                        }
                        glow::DEPTH_ATTACHMENT => {
                            gl.clear_buffer_f32_slice(glow::DEPTH, 0, &[depth])
                        }
                        glow::DEPTH_STENCIL_ATTACHMENT => {
                            let s = stencil as i32;
                            gl.clear_buffer_depth_stencil(glow::DEPTH_STENCIL, 0, depth, s)
                        }
                        _ => gl.clear_buffer_i32_slice(glow::STENCIL, 0, &[stencil as i32]),
                    }
                }
            }
            gl.framebuffer_texture(target, at, None, 0);
            gl.bind_framebuffer(target, None);
            gl.enable(glow::FRAMEBUFFER_SRGB);
            gl.depth_mask(self.state.is_some_and(|d| d.write));
            self.stencil();
            if let Some(p) = self.render {
                self.colors(p);
            }
        }
    }
}
