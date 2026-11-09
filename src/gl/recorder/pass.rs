use super::Recorder;
use crate::Error;
use crate::Texture;
use crate::{Format, LoadOp, StoreOp, slot};
use glow::HasContext;

const KEY: usize = 1 + 2 * (slot::COLORS as usize + 1);

fn is_srgb(f: Format) -> bool {
    matches!(f.gl()[0], glow::SRGB8 | glow::SRGB8_ALPHA8)
}

fn encode(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

impl Recorder<'_> {
    fn attach(&self, target: u32, point: u32, texture: Texture, layers: u32) {
        let t = self.res.textures.get(texture);
        let gl = self.gl;
        unsafe {
            match t.target {
                _ if layers > 1 => gl.framebuffer_texture(target, point, t.raw, 0),
                glow::TEXTURE_2D | glow::TEXTURE_2D_MULTISAMPLE | glow::TEXTURE_1D => {
                    gl.framebuffer_texture(target, point, t.raw, 0)
                }
                glow::TEXTURE_CUBE_MAP => gl.framebuffer_texture_2d(
                    target,
                    point,
                    glow::TEXTURE_CUBE_MAP_POSITIVE_X,
                    t.raw,
                    0,
                ),
                _ => gl.framebuffer_texture_layer(target, point, t.raw, 0, 0),
            }
        }
    }

    fn default(&self) -> bool {
        self.colors
            .iter()
            .any(|c| self.res.textures.get(c.texture).raw.is_none())
    }

    fn srgb(&self) -> bool {
        self.colors.iter().all(|c| {
            let t = self.res.textures.get(c.texture);
            t.raw.is_some() || is_srgb(t.format)
        })
    }

    fn bind(&mut self, layers: u32) -> Result<(), Error> {
        let gl = self.gl;
        let res = self.res;
        if self.default() {
            unsafe { gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None) };
            self.pass = None;
            return Ok(());
        }
        let raw = |t: Texture| res.textures.get(t).raw.map_or(0, |r| r.0.get());
        let mut key = [0; KEY];
        key[0] = (layers > 1) as u32;
        let mut n = 1;
        for i in 0..slot::COLORS {
            if let Some(c) = self.colors.iter().find(|c| c.index == i) {
                key[n..n + 2].copy_from_slice(&[glow::COLOR_ATTACHMENT0 + i, raw(c.texture)]);
                n += 2;
            }
        }
        let depth = self.depth.map(|d| {
            let point = match Format::gl(res.textures.get(d.texture).format)[1] {
                glow::DEPTH_STENCIL => glow::DEPTH_STENCIL_ATTACHMENT,
                glow::STENCIL_INDEX => glow::STENCIL_ATTACHMENT,
                _ => glow::DEPTH_ATTACHMENT,
            };
            key[n..n + 2].copy_from_slice(&[point, raw(d.texture)]);
            n += 2;
            (point, d.texture)
        });
        let key = &key[..n];
        let found = res
            .framebuffers
            .borrow()
            .iter()
            .find(|(k, _)| k == key)
            .map(|&(_, f)| f);
        if let Some(f) = found {
            unsafe { gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f)) };
            self.pass = Some(f);
            return Ok(());
        }
        let target = glow::DRAW_FRAMEBUFFER;
        unsafe {
            let f = gl.create_framebuffer().map_err(Error::Driver)?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(f));
            gl.read_buffer(glow::NONE);
            let mut draws = [glow::NONE; slot::COLORS as usize];
            let mut count = 0;
            for c in &self.colors {
                self.attach(target, glow::COLOR_ATTACHMENT0 + c.index, c.texture, layers);
                draws[c.index as usize] = glow::COLOR_ATTACHMENT0 + c.index;
                count = count.max(c.index as usize + 1);
            }
            gl.draw_buffers(&draws[..count]);
            if let Some((point, texture)) = depth {
                self.attach(target, point, texture, layers);
            }
            if gl.check_framebuffer_status(target) != glow::FRAMEBUFFER_COMPLETE {
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.delete_framebuffer(f);
                return Err(Error::Invalid(
                    "render targets form an incomplete framebuffer".into(),
                ));
            }
            res.framebuffers.borrow_mut().push((key.to_vec(), f));
            self.pass = Some(f);
        }
        Ok(())
    }

    pub fn begin_render(&mut self, width: u32, height: u32, layers: u32) -> Result<(), Error> {
        let gl = self.gl;
        let res = self.res;
        self.bind(layers)?;
        let default = self.default();
        let clears = self.colors.iter().any(|c| c.load == LoadOp::Clear)
            || self
                .depth
                .is_some_and(|d| d.load == LoadOp::Clear || d.stencil_load == LoadOp::Clear);
        if clears {
            self.enable(glow::SCISSOR_TEST, false);
            self.enable(glow::FRAMEBUFFER_SRGB, false);
            self.blended = None;
            self.stenciled = None;
            unsafe {
                gl.color_mask(true, true, true, true);
                gl.depth_mask(true);
                gl.stencil_mask(0xff);
                for c in &self.colors {
                    if c.load != LoadOp::Clear {
                        continue;
                    }
                    let slot = if default { 0 } else { c.index };
                    let clear = c.clear;
                    let f = res.textures.get(c.texture).format;
                    if f.sint() {
                        let values = clear.map(|v| v as i32);
                        gl.clear_buffer_i32_slice(glow::COLOR, slot, &values);
                    } else if f.uint() {
                        let values = clear.map(|v| v as u32);
                        gl.clear_buffer_u32_slice(glow::COLOR, slot, &values);
                    } else if is_srgb(f) {
                        let [r, g, b, a] = c.clear;
                        let encoded = [encode(r), encode(g), encode(b), a];
                        gl.clear_buffer_f32_slice(glow::COLOR, slot, &encoded);
                    } else {
                        gl.clear_buffer_f32_slice(glow::COLOR, slot, &clear);
                    }
                }
                if let Some(d) = &self.depth {
                    if d.load == LoadOp::Clear {
                        gl.clear_buffer_f32_slice(glow::DEPTH, 0, &[d.clear_depth]);
                    }
                    if d.stencil_load == LoadOp::Clear {
                        gl.clear_buffer_i32_slice(glow::STENCIL, 0, &[d.clear_stencil as i32]);
                    }
                }
                gl.depth_mask(self.state.is_some_and(|d| d.write));
            }
            if let Some(p) = self.render {
                self.apply_blend_state(p);
            }
            self.apply_stencil();
        }
        let srgb = self.srgb();
        self.enable(glow::FRAMEBUFFER_SRGB, srgb);
        unsafe {
            gl.viewport(0, 0, width as i32, height as i32);
            gl.depth_range(0.0, 1.0);
        }
        self.rendering = true;
        self.extent = (width as i32, height as i32);
        self.views.clear();
        self.views.push([0.0, 0.0, width as f32, height as f32]);
        self.scissors.clear();
        self.apply_scissors();
        self.layers = layers;
        Ok(())
    }

    pub fn end_render(&mut self) -> Result<(), Error> {
        let gl = self.gl;
        let target = glow::DRAW_FRAMEBUFFER;
        let default = self.default();
        let mut discard = [0; slot::COLORS as usize + 1];
        let mut n = 0;
        for c in self.colors.iter().filter(|c| c.store == StoreOp::Discard) {
            discard[n] = if default {
                glow::COLOR
            } else {
                glow::COLOR_ATTACHMENT0 + c.index
            };
            n += 1;
        }
        if self.depth.is_some_and(|d| d.store == StoreOp::Discard) {
            discard[n] = if default {
                glow::DEPTH
            } else {
                glow::DEPTH_ATTACHMENT
            };
            n += 1;
        }
        let (w, h) = self.extent;
        let resolves = self.colors.iter().any(|c| c.resolve.is_some());
        if resolves {
            self.enable(glow::SCISSOR_TEST, false);
        }
        unsafe {
            for c in &self.colors {
                let Some(resolve) = c.resolve else {
                    continue;
                };
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, self.pass);
                gl.read_buffer(glow::COLOR_ATTACHMENT0 + c.index);
                gl.bind_framebuffer(target, Some(self.read));
                self.attach(target, glow::COLOR_ATTACHMENT0, resolve, 1);
                gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
                gl.blit_framebuffer(
                    0,
                    0,
                    w,
                    h,
                    0,
                    0,
                    w,
                    h,
                    glow::COLOR_BUFFER_BIT,
                    glow::NEAREST,
                );
                gl.framebuffer_texture_2d(
                    target,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    None,
                    0,
                );
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                gl.bind_framebuffer(target, self.pass);
            }
            if let Some(invalidate) = self.extra.invalidate
                && n > 0
            {
                invalidate(target, n as i32, discard.as_ptr());
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }
        self.pass = None;
        self.clear_pass();
        self.rendering = false;
        Ok(())
    }
}
