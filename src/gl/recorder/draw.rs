use super::{Recorder, is_integer_format};
use crate::Buffer;
use crate::Error;
use crate::gl::pipeline::{Pipeline, attribute_size};
use crate::slot;
use glow::HasContext;

impl Recorder<'_> {
    pub fn bind_draw_state(&mut self, render: bool) -> Result<bool, Error> {
        let gl = self.gl;
        let program = if render {
            self.render.map(|p| &p.program)
        } else {
            self.program
        };
        let program = program.expect("bind a pipeline before drawing or dispatching");
        unsafe {
            if self.used != Some(program.raw) {
                gl.use_program(Some(program.raw));
                self.used = Some(program.raw);
            }
            if let Some(host) = &mut self.host {
                host.save_units(gl, program.pairs.len());
            }
            for (unit, &(t, s)) in program.pairs.iter().enumerate() {
                let pair = (
                    self.textures[t as usize],
                    s.and_then(|s| self.samplers[s as usize]),
                );
                if self.units.len() <= unit {
                    self.units.resize(unit + 1, None);
                }
                if self.units[unit] == Some(pair) {
                    continue;
                }
                self.units[unit] = Some(pair);
                if let Some(texture) = pair.0 {
                    let texture = self.res.textures.get(texture);
                    gl.active_texture(glow::TEXTURE0 + unit as u32);
                    gl.bind_texture(texture.target, texture.raw);
                }
                let sampler = pair.1.map(|s| *self.res.samplers.get(s));
                gl.bind_sampler(unit as u32, sampler);
            }
            let Some(p) = self.render.filter(|_| render) else {
                return Ok(true);
            };
            if self.vao != Some(p.vao) {
                self.vao = Some(p.vao);
                self.fed = false;
                self.slots = [None; slot::VERTICES as usize];
                self.elements = None;
                gl.bind_vertex_array(Some(p.vao));
            }
            if p.baked {
                self.shift(p, 0);
            } else if !self.fed {
                self.fed = true;
                self.bind_attributes(p, 0);
            }
            if let Some((buffer, _, _)) = self.index {
                let raw = self.res.buffers.get(buffer).raw;
                if self.elements != Some(raw.0.get()) {
                    self.elements = Some(raw.0.get());
                    gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(raw));
                }
            }
        }
        Ok(true)
    }

    fn shift(&mut self, p: &Pipeline, instance: u32) {
        if !p.baked {
            self.bind_attributes(p, instance);
            return;
        }
        for (s, layout) in p.buffers.iter().enumerate() {
            if p.slots & 1 << s == 0 || (instance != 0 && !layout.instanced) {
                continue;
            }
            let Some((buffer, offset)) = self.vertices[s] else {
                continue;
            };
            let at = offset + instance as u64 * layout.stride as u64;
            let raw = self.res.buffers.get(buffer).raw.0.get();
            if self.slots[s] == Some((raw, at)) {
                continue;
            }
            self.slots[s] = Some((raw, at));
            let stride = layout.stride as i32;
            unsafe { self.extra.bind_vertex.unwrap()(s as u32, raw, at as isize, stride) };
        }
    }

    fn bind_attributes(&self, p: &Pipeline, instance: u32) {
        let gl = self.gl;
        unsafe {
            for a in &p.attributes {
                let Some((buffer, offset)) = self.vertices[a.buffer_slot as usize] else {
                    gl.disable_vertex_attrib_array(a.location);
                    continue;
                };
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.res.buffers.get(buffer).raw));
                let layout = p.buffers[a.buffer_slot as usize];
                let [_, _, ty] = a.format.gl();
                let size = attribute_size(a.format);
                let skip = if layout.instanced {
                    instance as u64 * layout.stride as u64
                } else {
                    0
                };
                let at = (offset + skip + a.offset as u64) as usize as *const std::ffi::c_void;
                let stride = layout.stride as i32;
                if is_integer_format(a.format) {
                    (self.extra.integer)(a.location, size, ty, stride, at);
                } else {
                    let normalized = ty != glow::FLOAT && ty != glow::HALF_FLOAT;
                    (self.extra.attribute)(a.location, size, ty, normalized as u8, stride, at);
                }
                gl.enable_vertex_attrib_array(a.location);
                gl.vertex_attrib_divisor(a.location, layout.instanced as u32);
            }
        }
    }

    pub fn draw_direct(
        &mut self,
        indexed: bool,
        count: u32,
        instances: u32,
        first: u32,
        base: i32,
        instance: u32,
    ) {
        let gl = self.gl;
        let Some(p) = self.render else {
            return;
        };
        let native = self.extra.elements_base.is_some();
        unsafe {
            let given = native && self.extra.parameters;
            p.program
                .set_instance(gl, if given { 0 } else { instance as i32 });
            if instance != 0 && !native {
                self.shift(p, instance);
            }
            let (count, instances) = (count as i32, instances as i32);
            match self.index.filter(|_| indexed) {
                Some((_, offset, kind)) => {
                    let size = if kind == glow::UNSIGNED_SHORT { 2 } else { 4 };
                    let at = (offset + first as u64 * size) as usize as *const _;
                    match self.extra.elements_base {
                        Some(draw) if instance != 0 => {
                            draw(self.mode, count, kind, at, instances, base, instance)
                        }
                        _ => (self.extra.elements)(self.mode, count, kind, at, instances, base),
                    }
                }
                None if indexed => {}
                None if instance != 0 && native => gl.draw_arrays_instanced_base_instance(
                    self.mode,
                    first as i32,
                    count,
                    instances,
                    instance,
                ),
                None => gl.draw_arrays_instanced(self.mode, first as i32, count, instances),
            }
            if instance != 0 && !native {
                self.shift(p, 0);
            }
        }
    }

    pub fn draw_mesh_tasks(&self, buffer: Buffer, offset: u64) {
        let raw = self.res.buffers.get(buffer).raw;
        unsafe {
            self.gl.bind_buffer(glow::DRAW_INDIRECT_BUFFER, Some(raw));
            self.extra.mesh_indirect.unwrap()(offset as isize);
        }
    }

    pub fn draw_indirect_buffer(&self, indexed: bool, buffer: Buffer, offset: u64) {
        let kind = self.index.map_or(0, |(_, _, k)| k);
        let raw = self.res.buffers.get(buffer).raw;
        let pointer = offset as usize as *const std::ffi::c_void;
        unsafe {
            if let Some(p) = self.render {
                p.program.set_instance(self.gl, 0);
            }
            self.gl.bind_buffer(glow::DRAW_INDIRECT_BUFFER, Some(raw));
            if indexed {
                self.extra.indexed_indirect.unwrap()(self.mode, kind, pointer);
            } else {
                self.extra.indirect.unwrap()(self.mode, pointer);
            }
        }
    }
}
