use super::{Recorder, integer};
use crate::{format, load, slot, store};
use glow::HasContext;

impl Recorder<'_> {
    fn attach(&self, target: u32, point: u32, texture: u32, layers: u32) {
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

    pub fn begin_render(&mut self, width: u32, height: u32, layers: u32) -> Result<(), String> {
        let gl = self.gl;
        let res = self.res;
        unsafe {
            let default = self
                .colors
                .iter()
                .any(|c| res.textures.get(c.texture).raw.is_none());
            let target = glow::DRAW_FRAMEBUFFER;
            if default {
                gl.bind_framebuffer(target, None);
            } else {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
                gl.read_buffer(glow::NONE);
                for i in 0..slot::COLORS {
                    gl.framebuffer_texture_2d(
                        target,
                        glow::COLOR_ATTACHMENT0 + i,
                        glow::TEXTURE_2D,
                        None,
                        0,
                    );
                }
                gl.framebuffer_texture_2d(
                    target,
                    glow::DEPTH_STENCIL_ATTACHMENT,
                    glow::TEXTURE_2D,
                    None,
                    0,
                );
                let mut draws = Vec::new();
                for c in &self.colors {
                    self.attach(target, glow::COLOR_ATTACHMENT0 + c.index, c.texture, layers);
                    draws.resize(draws.len().max(c.index as usize + 1), glow::NONE);
                    draws[c.index as usize] = glow::COLOR_ATTACHMENT0 + c.index;
                }
                gl.draw_buffers(&draws);
                if let Some(d) = &self.depth {
                    let point = match format::gl(res.textures.get(d.texture).format)[1] {
                        glow::DEPTH_STENCIL => glow::DEPTH_STENCIL_ATTACHMENT,
                        glow::STENCIL_INDEX => glow::STENCIL_ATTACHMENT,
                        _ => glow::DEPTH_ATTACHMENT,
                    };
                    self.attach(target, point, d.texture, layers);
                }
                if gl.check_framebuffer_status(target) != glow::FRAMEBUFFER_COMPLETE {
                    return Err("render targets form an incomplete framebuffer".into());
                }
            }
            gl.color_mask(true, true, true, true);
            gl.depth_mask(true);
            gl.stencil_mask(0xff);
            gl.disable(glow::SCISSOR_TEST);
            for c in &self.colors {
                if c.load != load::CLEAR {
                    continue;
                }
                let slot = if default { 0 } else { c.index };
                let clear = [c.r, c.g, c.b, c.a];
                let f = res.textures.get(c.texture).format;
                if format::sint(f) {
                    let values = clear.map(|v| v as i32);
                    gl.clear_buffer_i32_slice(glow::COLOR, slot, &values);
                } else if format::uint(f) {
                    let values = clear.map(|v| v as u32);
                    gl.clear_buffer_u32_slice(glow::COLOR, slot, &values);
                } else {
                    gl.clear_buffer_f32_slice(glow::COLOR, slot, &clear);
                }
            }
            if let Some(d) = &self.depth {
                if d.load == load::CLEAR {
                    gl.clear_buffer_f32_slice(glow::DEPTH, 0, &[d.depth]);
                }
                if d.stencil_load == load::CLEAR {
                    gl.clear_buffer_i32_slice(glow::STENCIL, 0, &[d.stencil as i32]);
                }
            }
            if let Some(p) = self.render {
                self.colors(p);
            }
            gl.depth_mask(self.state.is_some_and(|d| d.write));
            self.stencil();
            gl.viewport(0, 0, width as i32, height as i32);
            gl.depth_range(0.0, 1.0);
            gl.disable(glow::SCISSOR_TEST);
            self.rendering = true;
            self.extent = (width as i32, height as i32);
        }
        Ok(())
    }

    pub fn end_render(&mut self) {
        let gl = self.gl;
        let res = self.res;
        unsafe {
            let target = glow::DRAW_FRAMEBUFFER;
            let mut discard: Vec<u32> = self
                .colors
                .iter()
                .filter(|c| c.store == store::DONT_CARE)
                .map(|c| glow::COLOR_ATTACHMENT0 + c.index)
                .collect();
            if self
                .depth
                .as_ref()
                .is_some_and(|d| d.store == store::DONT_CARE)
            {
                discard.push(glow::DEPTH_ATTACHMENT);
            }
            let (w, h) = self.extent;
            for c in &self.colors {
                if c.resolve == !0 {
                    continue;
                }
                gl.disable(glow::SCISSOR_TEST);
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.fbo));
                gl.read_buffer(glow::COLOR_ATTACHMENT0 + c.index);
                gl.bind_framebuffer(target, Some(self.read));
                self.attach(target, glow::COLOR_ATTACHMENT0, c.resolve, 1);
                gl.draw_buffers(&[glow::COLOR_ATTACHMENT0]);
                let source = res.textures.get(c.texture);
                if integer(source.format) {
                    let state = (self.snapshot)();
                    let raw = source.raw.expect("a resolve source has a texture");
                    self.resolve
                        .draw(gl, raw, format::sint(source.format), (w, h));
                    state.restore(gl, self.extra);
                } else {
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
                }
                gl.framebuffer_texture_2d(
                    target,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    None,
                    0,
                );
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                gl.bind_framebuffer(target, Some(self.fbo));
            }
            if !discard.is_empty() && self.caps.storage {
                gl.invalidate_framebuffer(target, &discard);
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.colors.clear();
            self.depth = None;
            self.rendering = false;
        }
    }
}
