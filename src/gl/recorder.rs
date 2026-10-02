use crate::command;
use crate::gl::device::{PUSH, compare};
use crate::gl::extra::Extra;
use crate::gl::native::{Callback, Native};
use crate::gl::pipeline::{Pipeline, uniform};
use crate::gl::program::Program;
use crate::gl::resolve::Resolve;
use crate::gl::resources::Resources;
use crate::gl::state::{State, toggle};
use crate::register;
use crate::{
    Caps, Command, DepthInfo, access, cull, face, format, index, load, query, slot, store, topology,
};
use glow::HasContext;

const UNUSED: [u32; 19] = [
    glow::RASTERIZER_DISCARD,
    glow::COLOR_LOGIC_OP,
    glow::SAMPLE_COVERAGE,
    glow::SAMPLE_MASK,
    glow::SAMPLE_ALPHA_TO_ONE,
    glow::SAMPLE_ALPHA_TO_COVERAGE,
    glow::DEPTH_CLAMP,
    glow::DITHER,
    glow::PRIMITIVE_RESTART,
    glow::LINE_SMOOTH,
    glow::POLYGON_SMOOTH,
    glow::CLIP_DISTANCE0,
    glow::CLIP_DISTANCE1,
    glow::CLIP_DISTANCE2,
    glow::CLIP_DISTANCE3,
    glow::CLIP_DISTANCE4,
    glow::CLIP_DISTANCE5,
    glow::CLIP_DISTANCE6,
    glow::CLIP_DISTANCE7,
];

const OFFSETS: [u32; 3] = [
    glow::POLYGON_OFFSET_FILL,
    glow::POLYGON_OFFSET_LINE,
    glow::POLYGON_OFFSET_POINT,
];

struct Color {
    index: u32,
    texture: u32,
    load: u32,
    store: u32,
    clear: [f32; 4],
    resolve: u32,
}

struct Depth {
    texture: u32,
    load: u32,
    store: u32,
    depth: f32,
    stencil_load: u32,
    stencil: u32,
}

pub struct Recorder<'a> {
    gl: &'a glow::Context,
    extra: &'a Extra,
    res: &'a Resources,
    caps: &'a Caps,
    push: glow::Buffer,
    fbo: glow::Framebuffer,
    read: glow::Framebuffer,
    resolve: &'a Resolve,
    snapshot: &'a dyn Fn() -> State,
    program: Option<&'a Program>,
    render: Option<&'a Pipeline>,
    colors: Vec<Color>,
    depth: Option<Depth>,
    data: [u8; PUSH],
    size: usize,
    dirty: bool,
    textures: [Option<u32>; slot::TEXTURES as usize],
    samplers: [Option<u32>; slot::SAMPLERS as usize],
    vertices: [Option<(u32, u64)>; slot::VERTICES as usize],
    index: Option<(u32, u64, u32)>,
    mode: u32,
    state: Option<DepthInfo>,
    references: (u32, u32),
    extent: (i32, i32),
    rendering: bool,
    query: Option<u32>,
    labels: u32,
    natives: &'a [Callback],
}

fn mode(t: u32) -> u32 {
    match t {
        topology::POINT => glow::POINTS,
        topology::LINE => glow::LINES,
        topology::LINE_STRIP => glow::LINE_STRIP,
        topology::TRIANGLE => glow::TRIANGLES,
        topology::TRIANGLE_STRIP => glow::TRIANGLE_STRIP,
        t => panic!("unknown topology {t}"),
    }
}

fn stencil_op(op: u32) -> u32 {
    [
        glow::KEEP,
        glow::ZERO,
        glow::REPLACE,
        glow::INCR,
        glow::DECR,
        glow::INVERT,
        glow::INCR_WRAP,
        glow::DECR_WRAP,
    ][op as usize]
}

fn factor(f: u32) -> u32 {
    [
        glow::ZERO,
        glow::ONE,
        glow::SRC_COLOR,
        glow::ONE_MINUS_SRC_COLOR,
        glow::DST_COLOR,
        glow::ONE_MINUS_DST_COLOR,
        glow::SRC_ALPHA,
        glow::ONE_MINUS_SRC_ALPHA,
        glow::DST_ALPHA,
        glow::ONE_MINUS_DST_ALPHA,
        glow::CONSTANT_COLOR,
        glow::ONE_MINUS_CONSTANT_COLOR,
        glow::CONSTANT_ALPHA,
        glow::ONE_MINUS_CONSTANT_ALPHA,
        glow::SRC_ALPHA_SATURATE,
        glow::SRC1_COLOR,
        glow::ONE_MINUS_SRC1_COLOR,
        glow::SRC1_ALPHA,
        glow::ONE_MINUS_SRC1_ALPHA,
    ][f as usize]
}

fn equation(op: u32) -> u32 {
    [
        glow::FUNC_ADD,
        glow::FUNC_SUBTRACT,
        glow::FUNC_REVERSE_SUBTRACT,
        glow::MIN,
        glow::MAX,
    ][op as usize]
}

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

fn integer(f: u32) -> bool {
    format::uint(f) || format::sint(f)
}

fn barriers(a: u32) -> u32 {
    let map = [
        (
            access::INDIRECT_READ,
            glow::COMMAND_BARRIER_BIT | glow::BUFFER_UPDATE_BARRIER_BIT,
        ),
        (access::INDEX_READ, glow::ELEMENT_ARRAY_BARRIER_BIT),
        (access::VERTEX_READ, glow::VERTEX_ATTRIB_ARRAY_BARRIER_BIT),
        (access::UNIFORM_READ, glow::UNIFORM_BARRIER_BIT),
        (
            access::SHADER_READ | access::SHADER_WRITE,
            glow::SHADER_STORAGE_BARRIER_BIT
                | glow::SHADER_IMAGE_ACCESS_BARRIER_BIT
                | glow::TEXTURE_FETCH_BARRIER_BIT,
        ),
        (
            access::COLOR_READ | access::COLOR_WRITE | access::DEPTH_READ | access::DEPTH_WRITE,
            glow::FRAMEBUFFER_BARRIER_BIT,
        ),
        (
            access::COPY_READ | access::COPY_WRITE,
            glow::BUFFER_UPDATE_BARRIER_BIT
                | glow::TEXTURE_UPDATE_BARRIER_BIT
                | glow::PIXEL_BUFFER_BARRIER_BIT
                | glow::FRAMEBUFFER_BARRIER_BIT,
        ),
        (
            access::HOST_READ | access::HOST_WRITE,
            glow::BUFFER_UPDATE_BARRIER_BIT
                | glow::PIXEL_BUFFER_BARRIER_BIT
                | glow::CLIENT_MAPPED_BUFFER_BARRIER_BIT,
        ),
        (
            access::MEMORY_READ | access::MEMORY_WRITE,
            glow::ALL_BARRIER_BITS,
        ),
    ];
    map.iter()
        .filter(|(b, _)| a & b != 0)
        .fold(0, |m, (_, g)| m | g)
}

impl<'a> Recorder<'a> {
    pub fn new(
        gl: &'a glow::Context,
        extra: &'a Extra,
        res: &'a Resources,
        caps: &'a Caps,
        (push, fbo, read): (glow::Buffer, glow::Framebuffer, glow::Framebuffer),
        (resolve, snapshot): (&'a Resolve, &'a dyn Fn() -> State),
        natives: &'a [Callback],
    ) -> Self {
        Self {
            gl,
            extra,
            res,
            caps,
            push,
            fbo,
            read,
            resolve,
            snapshot,
            program: None,
            render: None,
            colors: Vec::new(),
            depth: None,
            data: [0; PUSH],
            size: 0,
            dirty: false,
            textures: [None; slot::TEXTURES as usize],
            samplers: [None; slot::SAMPLERS as usize],
            vertices: [None; slot::VERTICES as usize],
            index: None,
            mode: glow::TRIANGLES,
            state: None,
            references: (0, 0),
            extent: (0, 0),
            rendering: false,
            query: None,
            labels: 0,
            natives,
        }
    }

    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), String> {
        unsafe {
            self.gl
                .bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(self.push))
        };
        self.reset();
        while !bytes.is_empty() {
            self.step(Command::read(&mut bytes)?)?;
        }
        Ok(())
    }

    pub fn finish(&self) {
        let gl = self.gl;
        unsafe {
            if let Some(target) = self.query {
                gl.end_query(target);
            }
            for _ in 0..self.labels {
                gl.pop_debug_group();
            }
        }
    }

    fn prepare(&mut self, render: bool) -> bool {
        let gl = self.gl;
        let program = if render {
            self.render.map(|p| &p.program)
        } else {
            self.program
        };
        let Some(program) = program else {
            return false;
        };
        unsafe {
            gl.use_program(Some(program.raw));
            if self.dirty {
                gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.push));
                gl.buffer_sub_data_u8_slice(glow::UNIFORM_BUFFER, 0, &self.data[..self.size]);
                gl.bind_buffer(glow::UNIFORM_BUFFER, None);
                self.dirty = false;
            }
            for (unit, &(t, s)) in program.pairs.iter().enumerate() {
                if let Some(texture) = self.textures[t as usize] {
                    let texture = self.res.textures.get(texture);
                    gl.active_texture(glow::TEXTURE0 + unit as u32);
                    gl.bind_texture(texture.target, texture.raw);
                }
                let sampler = s.and_then(|s| self.samplers[s as usize]);
                let sampler = sampler.map(|s| *self.res.samplers.get(s));
                gl.bind_sampler(unit as u32, sampler);
            }
            let Some(p) = self.render.filter(|_| render) else {
                return true;
            };
            gl.bind_vertex_array(Some(p.vao));
            self.attributes(p, 0);
            if let Some((buffer, _, _)) = self.index {
                gl.bind_buffer(
                    glow::ELEMENT_ARRAY_BUFFER,
                    Some(self.res.buffers.get(buffer).raw),
                );
            }
        }
        true
    }

    fn attributes(&self, p: &Pipeline, instance: u32) {
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
                if integer(a.format) {
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

    fn direct(
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
            let base = if indexed { base } else { first as i32 };
            if let Some(l) = &p.program.base {
                gl.uniform_1_i32(Some(l), base);
            }
            if let Some(l) = &p.program.instance {
                gl.uniform_1_i32(Some(l), instance as i32);
            }
            if instance != 0 && !native {
                self.attributes(p, instance);
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
                self.attributes(p, 0);
            }
        }
    }

    fn indirect(
        &self,
        indexed: bool,
        buffer: u32,
        offset: u64,
        count: u32,
        stride: u32,
    ) -> Result<(), String> {
        let gl = self.gl;
        let Some(p) = self.render.filter(|_| !indexed || self.index.is_some()) else {
            return Ok(());
        };
        if p.program.base.is_some() {
            return Err(
                "indirect draws of a shader that reads SV_VertexID are not supported on GL".into(),
            );
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
            match (indexed, count) {
                (_, 0) => {}
                (true, 1) => extra.indexed_indirect.unwrap()(self.mode, kind, pointer),
                (false, 1) => extra.indirect.unwrap()(self.mode, pointer),
                (true, _) => extra.multi_indexed.unwrap()(self.mode, kind, pointer, n, step),
                (false, _) => extra.multi.unwrap()(self.mode, pointer, n, step),
            }
        }
        Ok(())
    }

    fn range(&self, target: u32, index: u32, buffer: u32, offset: u64, size: u64) {
        let b = self.res.buffers.get(buffer);
        let size = if size == !0 { b.size - offset } else { size };
        unsafe { (self.extra.range)(target, index, b.raw.0.get(), offset as isize, size as isize) };
    }

    fn colors(&self, p: &Pipeline) {
        let gl = self.gl;
        let uniform = uniform(&p.colors);
        unsafe {
            for (i, c) in p.colors.iter().enumerate() {
                let i = i as u32;
                let m = c.mask;
                gl.color_mask_draw_buffer(i, m & 1 != 0, m & 2 != 0, m & 4 != 0, m & 8 != 0);
                if !c.blend {
                    gl.disable_draw_buffer(glow::BLEND, i);
                    continue;
                }
                gl.enable_draw_buffer(glow::BLEND, i);
                let (src, dst) = (factor(c.src), factor(c.dst));
                let (src_alpha, dst_alpha) = (factor(c.src_alpha), factor(c.dst_alpha));
                let (op, op_alpha) = (equation(c.op), equation(c.op_alpha));
                if uniform {
                    gl.blend_func_separate(src, dst, src_alpha, dst_alpha);
                    gl.blend_equation_separate(op, op_alpha);
                } else {
                    gl.blend_func_separate_draw_buffer(i, src, dst, src_alpha, dst_alpha);
                    gl.blend_equation_separate_draw_buffer(i, op, op_alpha);
                }
            }
        }
    }

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

    fn reset(&mut self) {
        let gl = self.gl;
        unsafe {
            (self.extra.provoking)(glow::FIRST_VERTEX_CONVENTION);
            gl.enable(glow::FRAMEBUFFER_SRGB);
            gl.enable(glow::TEXTURE_CUBE_MAP_SEAMLESS);
            gl.enable(glow::PROGRAM_POINT_SIZE);
            gl.disable(glow::CULL_FACE);
            gl.front_face(glow::CW);
            for mode in OFFSETS {
                gl.disable(mode);
            }
            gl.polygon_offset(0.0, 0.0);
            gl.blend_color(0.0, 0.0, 0.0, 0.0);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::STENCIL_TEST);
            gl.line_width(1.0);
            for e in UNUSED {
                gl.disable(e);
            }
            gl.enable(glow::MULTISAMPLE);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            (self.extra.clamp_color)(glow::CLAMP_READ_COLOR, glow::FIXED_ONLY);
            for i in 0..slot::COLORS {
                gl.color_mask_draw_buffer(i, true, true, true, true);
                gl.disable_draw_buffer(glow::BLEND, i);
            }
        }
    }

    fn stencil(&self) {
        let Some(d) = self.state.filter(|d| d.stencil) else {
            return;
        };
        let gl = self.gl;
        unsafe {
            for (side, f, reference) in [
                (glow::FRONT, d.front, self.references.0),
                (glow::BACK, d.back, self.references.1),
            ] {
                gl.stencil_op_separate(
                    side,
                    stencil_op(f.fail),
                    stencil_op(f.depth_fail),
                    stencil_op(f.pass),
                );
                gl.stencil_func_separate(side, compare(f.compare), reference as i32, f.read);
                gl.stencil_mask_separate(side, f.write);
            }
        }
    }

    fn step(&mut self, command: Command) -> Result<(), String> {
        let gl = self.gl;
        let res = self.res;
        unsafe {
            match command {
                Command::BeginCompute
                | Command::EndCompute
                | Command::OcclusionHeap { .. }
                | Command::ResetQueries { .. }
                | Command::SetDepthBounds { .. } => {}
                Command::BindPipeline { pipeline } => {
                    let p = res.pipelines.get(pipeline);
                    self.program = Some(p);
                }
                Command::PushConstants { offset, data } => {
                    let at = offset as usize;
                    self.data[at..at + data.len()].copy_from_slice(data);
                    self.size = self.size.max(at + data.len());
                    self.dirty = true;
                }
                Command::Dispatch { x, y, z } => {
                    if self.prepare(false) {
                        gl.dispatch_compute(x, y, z);
                    }
                }
                Command::DispatchIndirect { buffer, offset } => {
                    if self.prepare(false) {
                        gl.bind_buffer(
                            glow::DISPATCH_INDIRECT_BUFFER,
                            Some(res.buffers.get(buffer).raw),
                        );
                        self.extra.dispatch.unwrap()(offset as isize);
                    }
                }
                Command::CopyBuffer {
                    src,
                    src_offset,
                    dst,
                    dst_offset,
                    size,
                } => {
                    gl.bind_buffer(glow::COPY_READ_BUFFER, Some(res.buffers.get(src).raw));
                    gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(res.buffers.get(dst).raw));
                    (self.extra.copy)(
                        glow::COPY_READ_BUFFER,
                        glow::COPY_WRITE_BUFFER,
                        src_offset as isize,
                        dst_offset as isize,
                        size as isize,
                    );
                }
                Command::FillBuffer {
                    buffer,
                    offset,
                    size,
                    value,
                } => {
                    let b = res.buffers.get(buffer);
                    let size = if size == !0 { b.size - offset } else { size };
                    let target = glow::COPY_WRITE_BUFFER;
                    gl.bind_buffer(target, Some(b.raw));
                    match self.extra.clear {
                        Some(clear) => clear(
                            target,
                            glow::R8UI,
                            offset as isize,
                            size as isize,
                            glow::RED_INTEGER,
                            glow::UNSIGNED_BYTE,
                            (&raw const value).cast(),
                        ),
                        None => {
                            let data = vec![value; size as usize];
                            let (at, size) = (offset as isize, size as isize);
                            (self.extra.sub)(target, at, size, data.as_ptr().cast());
                        }
                    }
                }
                Command::Barrier {
                    src_access,
                    dst_access,
                    ..
                }
                | Command::QueueBarrier {
                    src_access,
                    dst_access,
                    ..
                }
                | Command::TextureBarrier {
                    src_access,
                    dst_access,
                    ..
                } => {
                    if src_access & (access::SHADER_WRITE | access::MEMORY_WRITE) != 0
                        && self.caps.storage
                    {
                        gl.memory_barrier(barriers(dst_access));
                    }
                }
                Command::BeginLabel { text } => {
                    gl.push_debug_group(glow::DEBUG_SOURCE_APPLICATION, 0, text);
                    self.labels += 1;
                }
                Command::EndLabel => {
                    if self.labels > 0 {
                        gl.pop_debug_group();
                        self.labels -= 1;
                    }
                }
                Command::CopyBufferToTexture {
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
                } => {
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
                                        (e.compressed2)(
                                            target, mip, x, top, w, h, internal, size, at,
                                        )
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
                            _ => {
                                (e.image3)(t.target, mip, x, y, layer as i32, w, h, 1, f, ty, data)
                            }
                        }
                    }
                    gl.bind_texture(t.target, None);
                    gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
                    gl.pixel_store_i32(glow::UNPACK_IMAGE_HEIGHT, 0);
                    gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
                }
                Command::CopyTextureToBuffer {
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
                } => {
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
                            _ => gl.framebuffer_texture_layer(
                                target,
                                point,
                                t.raw,
                                mip,
                                layer as i32,
                            ),
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
                Command::ColorAttachment {
                    index,
                    texture,
                    load,
                    store,
                    r,
                    g,
                    b,
                    a,
                    resolve,
                    ..
                } => {
                    self.colors.retain(|c| c.index != index);
                    self.colors.push(Color {
                        index,
                        texture,
                        load,
                        store,
                        clear: [r, g, b, a],
                        resolve,
                    });
                }
                Command::DepthAttachment {
                    texture,
                    load,
                    store,
                    depth,
                    stencil_load,
                    stencil,
                    ..
                } => {
                    self.depth = Some(Depth {
                        texture,
                        load,
                        store,
                        depth,
                        stencil_load,
                        stencil,
                    });
                }
                Command::BeginRender {
                    width,
                    height,
                    layers,
                } => {
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
                            self.attach(
                                target,
                                glow::COLOR_ATTACHMENT0 + c.index,
                                c.texture,
                                layers,
                            );
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
                        let f = res.textures.get(c.texture).format;
                        if format::sint(f) {
                            let values = c.clear.map(|v| v as i32);
                            gl.clear_buffer_i32_slice(glow::COLOR, slot, &values);
                        } else if format::uint(f) {
                            let values = c.clear.map(|v| v as u32);
                            gl.clear_buffer_u32_slice(glow::COLOR, slot, &values);
                        } else {
                            gl.clear_buffer_f32_slice(glow::COLOR, slot, &c.clear);
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
                Command::EndRender => {
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
                Command::BindRenderPipeline { pipeline } => {
                    let p = res.renders.get(pipeline);
                    self.render = Some(p);
                    self.mode = mode(p.topology);
                    let fill = if p.fill == crate::fill::LINE {
                        glow::LINE
                    } else {
                        glow::FILL
                    };
                    gl.polygon_mode(glow::FRONT_AND_BACK, fill);
                    toggle(gl, glow::DEPTH_CLAMP, p.clamp);
                    toggle(gl, glow::SAMPLE_ALPHA_TO_COVERAGE, p.coverage);
                    self.colors(p);
                }
                Command::SetViewport {
                    x,
                    y,
                    width,
                    height,
                    near,
                    far,
                } => {
                    gl.viewport(x as i32, y as i32, width as i32, height as i32);
                    gl.depth_range(near as f64, far as f64);
                }
                Command::SetScissor {
                    x,
                    y,
                    width,
                    height,
                } => {
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(x as i32, y as i32, width as i32, height as i32);
                }
                Command::SetCull { cull: c } => match c {
                    cull::NONE => gl.disable(glow::CULL_FACE),
                    cull::FRONT | cull::BACK => {
                        gl.enable(glow::CULL_FACE);
                        gl.cull_face(if c == cull::FRONT {
                            glow::FRONT
                        } else {
                            glow::BACK
                        });
                    }
                    c => panic!("unknown cull {c}"),
                },
                Command::SetFace { face: f } => gl.front_face(match f {
                    face::CCW => glow::CW,
                    face::CW => glow::CCW,
                    f => panic!("unknown face {f}"),
                }),
                Command::SetTopology { topology } => self.mode = mode(topology),
                Command::SetDepthState { state } => {
                    let d = *res.depths.get(state);
                    toggle(
                        gl,
                        glow::DEPTH_TEST,
                        d.compare != crate::compare::NONE || d.write,
                    );
                    gl.depth_func(compare(d.compare));
                    gl.depth_mask(d.write);
                    toggle(gl, glow::STENCIL_TEST, d.stencil);
                    self.state = Some(d);
                    self.stencil();
                }
                Command::SetStencilReference { front, back } => {
                    self.references = (front, back);
                    self.stencil();
                }
                Command::SetDepthBias {
                    constant,
                    slope,
                    clamp,
                } => {
                    let on = constant != 0.0 || slope != 0.0;
                    for mode in OFFSETS {
                        toggle(gl, mode, on);
                    }
                    match self.extra.offset_clamp {
                        Some(offset) => offset(slope, constant, clamp),
                        None => gl.polygon_offset(slope, constant),
                    }
                }
                Command::SetBlendColor { r, g, b, a } => gl.blend_color(r, g, b, a),
                Command::BindVertexBuffer {
                    slot,
                    buffer,
                    offset,
                } => self.vertices[slot as usize] = Some((buffer, offset)),
                Command::BindIndexBuffer {
                    buffer,
                    offset,
                    kind,
                } => {
                    let kind = match kind {
                        index::U16 => glow::UNSIGNED_SHORT,
                        index::U32 => glow::UNSIGNED_INT,
                        k => panic!("unknown index {k}"),
                    };
                    self.index = Some((buffer, offset, kind));
                }
                Command::Draw {
                    vertices,
                    instances,
                    first_vertex,
                    first_instance,
                } => {
                    if self.prepare(true) {
                        self.direct(false, vertices, instances, first_vertex, 0, first_instance);
                    }
                }
                Command::DrawIndexed {
                    indices,
                    instances,
                    first_index,
                    base_vertex,
                    first_instance,
                } => {
                    if self.prepare(true) {
                        self.direct(
                            true,
                            indices,
                            instances,
                            first_index,
                            base_vertex,
                            first_instance,
                        );
                    }
                }
                Command::DrawIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if self.prepare(true) {
                        self.indirect(false, buffer, offset, count, stride)?;
                    }
                }
                Command::DrawIndexedIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if self.prepare(true) {
                        self.indirect(true, buffer, offset, count, stride)?;
                    }
                }
                Command::DrawMesh { x, y, z } => {
                    if self.prepare(true) {
                        self.extra.mesh.unwrap()(x, y, z);
                    }
                }
                Command::DrawMeshIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if !self.prepare(true) {
                        return Ok(());
                    }
                    gl.bind_buffer(
                        glow::DRAW_INDIRECT_BUFFER,
                        Some(res.buffers.get(buffer).raw),
                    );
                    if count == 1 {
                        self.extra.mesh_indirect.unwrap()(offset as isize);
                    } else {
                        self.extra.mesh_multi.unwrap()(
                            offset as isize,
                            count as i32,
                            stride as i32,
                        );
                    }
                }
                Command::WriteTimestamp { heap, index, .. } => {
                    gl.query_counter(res.queries.get(heap).ids[index as usize], glow::TIMESTAMP);
                }
                Command::BeginQuery { heap, index } => {
                    let q = res.queries.get(heap);
                    let target = if q.kind == query::PRECISE {
                        glow::SAMPLES_PASSED
                    } else {
                        glow::ANY_SAMPLES_PASSED
                    };
                    gl.begin_query(target, q.ids[index as usize]);
                    self.query = Some(target);
                }
                Command::EndQuery { heap, .. } => {
                    let q = res.queries.get(heap);
                    let target = if q.kind == query::PRECISE {
                        glow::SAMPLES_PASSED
                    } else {
                        glow::ANY_SAMPLES_PASSED
                    };
                    if self.query.take() == Some(target) {
                        gl.end_query(target);
                    }
                }
                Command::ResolveQueries {
                    heap,
                    first,
                    count,
                    buffer,
                    offset,
                } => {
                    let q = res.queries.get(heap);
                    let raw = res.buffers.get(buffer).raw;
                    gl.bind_buffer(glow::QUERY_BUFFER, Some(raw));
                    for i in 0..count {
                        let id = q.ids[(first + i) as usize];
                        let at = (offset + i as u64 * 8) as usize;
                        gl.get_query_parameter_u64_with_offset(id, glow::QUERY_RESULT, at);
                    }
                    gl.bind_buffer(glow::QUERY_BUFFER, None);
                }
                Command::SetViewports { viewports } => {
                    let views = command::viewports(viewports);
                    match views.as_slice() {
                        [v] => {
                            gl.viewport(v.x as i32, v.y as i32, v.width as i32, v.height as i32);
                            gl.depth_range(v.near as f64, v.far as f64);
                        }
                        _ => {
                            let rects: Vec<[f32; 4]> = views
                                .iter()
                                .map(|v| [v.x, v.y, v.width, v.height])
                                .collect();
                            let ranges: Vec<[f64; 2]> = views
                                .iter()
                                .map(|v| [v.near as f64, v.far as f64])
                                .collect();
                            gl.viewport_f32_slice(0, rects.len() as i32, &rects);
                            gl.depth_range_f64_slice(0, ranges.len() as i32, &ranges);
                        }
                    }
                }
                Command::SetScissors { scissors } => {
                    let rects: Vec<[i32; 4]> = command::scissors(scissors)
                        .iter()
                        .map(|r| [r.x as i32, r.y as i32, r.width as i32, r.height as i32])
                        .collect();
                    gl.enable(glow::SCISSOR_TEST);
                    match rects.as_slice() {
                        [r] => gl.scissor(r[0], r[1], r[2], r[3]),
                        _ => gl.scissor_slice(0, rects.len() as i32, &rects),
                    }
                }
                Command::BindUniform {
                    slot,
                    buffer,
                    offset,
                    size,
                } => self.range(
                    glow::UNIFORM_BUFFER,
                    register::UNIFORM + slot,
                    buffer,
                    offset,
                    size,
                ),
                Command::BindStorage {
                    slot,
                    buffer,
                    offset,
                    size,
                } => {
                    self.range(glow::SHADER_STORAGE_BUFFER, slot, buffer, offset, size);
                }
                Command::BindImage { slot, texture } => {
                    let t = res.textures.get(texture);
                    let layered = !matches!(t.target, glow::TEXTURE_2D | glow::TEXTURE_1D);
                    gl.bind_image_texture(
                        slot,
                        t.raw,
                        0,
                        layered,
                        0,
                        glow::READ_WRITE,
                        format::gl(t.format)[0],
                    );
                }
                Command::BindTexture { slot, texture } => {
                    self.textures[slot as usize] = Some(texture)
                }
                Command::BindSampler { slot, sampler } => {
                    self.samplers[slot as usize] = Some(sampler)
                }
                Command::Callback { index } => {
                    self.prepare(self.rendering);
                    self.natives[index as usize](&Native { gl });
                }
            }
        }
        Ok(())
    }
}
