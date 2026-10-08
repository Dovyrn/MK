use super::{Recorder, is_integer_format};
use crate::Buffer;
use crate::format;
use crate::gl::pipeline::Pipeline;
use glow::HasContext;

impl Recorder<'_> {
    pub fn bind_draw_state(&mut self, render: bool) -> Result<bool, String> {
        let gl = self.gl;
        let program = if render {
            self.render.map(|p| &p.program)
        } else {
            self.program
        };
        let Some(program) = program else {
            return Ok(false);
        };
        unsafe {
            if self.used != Some(program.raw) {
                gl.use_program(Some(program.raw));
                self.used = Some(program.raw);
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
            if self.fed == Some(p.vao) {
                return Ok(true);
            }
            self.fed = Some(p.vao);
            gl.bind_vertex_array(Some(p.vao));
            self.bind_attributes(p, 0);
            if let Some((buffer, _, _)) = self.index {
                gl.bind_buffer(
                    glow::ELEMENT_ARRAY_BUFFER,
                    Some(self.res.buffers.get(buffer).raw),
                );
            }
        }
        Ok(true)
    }

    fn bind_attributes(&self, p: &Pipeline, instance: u32) {
        let gl = self.gl;
        unsafe {
            for a in &p.attributes {
                let Some((buffer, offset)) = self.vertices[a.buffer as usize] else {
                    gl.disable_vertex_attrib_array(a.location);
                    continue;
                };
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.res.buffers.get(buffer).raw));
                let layout = p.buffers[a.buffer as usize];
                let [_, f, ty] = format::gl(a.format);
                let size = match f {
                    glow::RED | glow::RED_INTEGER => 1,
                    glow::RG | glow::RG_INTEGER => 2,
                    glow::RGB | glow::RGB_INTEGER => 3,
                    glow::BGRA => glow::BGRA as i32,
                    _ => 4,
                };
                let skip = if layout.instance {
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
                gl.vertex_attrib_divisor(a.location, layout.instance as u32);
            }
        }
    }

    pub fn draw_direct(
        &self,
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
            if let Some(l) = &p.program.instance {
                gl.uniform_1_i32(Some(l), instance as i32);
            }
            if instance != 0 && !native {
                self.bind_attributes(p, instance);
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
                self.bind_attributes(p, 0);
            }
        }
    }

    pub fn draw_mesh_tasks(
        &self,
        buffer: Buffer,
        offset: u64,
        count: u32,
        stride: u32,
        parameter: Option<(Buffer, u64)>,
    ) -> Result<(), String> {
        let gl = self.gl;
        let extra = self.extra;
        let raw = self.res.buffers.get(buffer).raw;
        unsafe {
            gl.bind_buffer(glow::DRAW_INDIRECT_BUFFER, Some(raw));
            if let Some((buffer, at)) = parameter {
                let draw = extra
                    .mesh_count
                    .ok_or("mesh count draws need glMultiDrawMeshTasksIndirectCountEXT")?;
                let raw = self.res.buffers.get(buffer).raw;
                gl.bind_buffer(glow::PARAMETER_BUFFER, Some(raw));
                draw(offset as isize, at as isize, count as i32, stride as i32);
            } else if count == 1 {
                extra.mesh_indirect.unwrap()(offset as isize);
            } else {
                extra.mesh_multi.unwrap()(offset as isize, count as i32, stride as i32);
            }
        }
        Ok(())
    }

    pub fn draw_indirect_buffer(
        &self,
        indexed: bool,
        buffer: Buffer,
        offset: u64,
        count: u32,
        stride: u32,
        parameter: Option<(Buffer, u64)>,
    ) {
        let gl = self.gl;
        if self.render.is_none() || indexed && self.index.is_none() {
            return;
        }
        let size = if indexed { 20 } else { 16 };
        let stride = if stride == 0 { size } else { stride };
        let kind = self.index.map_or(0, |(_, _, k)| k);
        let extra = self.extra;
        unsafe {
            gl.bind_buffer(
                glow::DRAW_INDIRECT_BUFFER,
                Some(self.res.buffers.get(buffer).raw),
            );
            let pointer = offset as usize as *const std::ffi::c_void;
            let (n, step) = (count as i32, stride as i32);
            if let Some((buffer, at)) = parameter {
                gl.bind_buffer(
                    glow::PARAMETER_BUFFER,
                    Some(self.res.buffers.get(buffer).raw),
                );
                let at = at as isize;
                if indexed {
                    extra.count_indexed.unwrap()(self.mode, kind, pointer, at, n, step);
                } else {
                    extra.count.unwrap()(self.mode, pointer, at, n, step);
                }
                return;
            }
            match (indexed, count) {
                (true, 1) => extra.indexed_indirect.unwrap()(self.mode, kind, pointer),
                (false, 1) => extra.indirect.unwrap()(self.mode, pointer),
                (true, _) => extra.multi_indexed.unwrap()(self.mode, kind, pointer, n, step),
                (false, _) => extra.multi.unwrap()(self.mode, pointer, n, step),
            }
        }
    }
}
