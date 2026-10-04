mod copy;
mod draw;
mod pass;
mod textures;

use crate::command;
use crate::gl::device::{PUSH, RING, compare};
use crate::gl::extra::Extra;
use crate::gl::native::{Callback, Native};
use crate::gl::pipeline::{Pipeline, uniform};
use crate::gl::program::Program;
use crate::gl::rates;
use crate::gl::resolve::Resolve;
use crate::gl::resources::Resources;
use crate::gl::state::{State, toggle};
use crate::register;
use crate::{
    Caps, ColorAttachment, Command, DepthAttachment, DepthInfo, access, combiner, cull, face,
    format, index, slot, topology,
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
    colors: Vec<ColorAttachment>,
    depth: Option<DepthAttachment>,
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
    views: Vec<[f32; 4]>,
    scissors: Vec<[i32; 4]>,
    layers: u32,
    shading: [u32; 4],
    rate: Option<(u32, u32, u32)>,
    rated: bool,
    rendering: bool,
    query: Option<u32>,
    labels: u32,
    natives: &'a [Callback],
    used: Option<glow::Program>,
    ring: usize,
    units: Vec<Option<(Option<u32>, Option<u32>)>>,
    fed: Option<glow::VertexArray>,
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
            views: Vec::new(),
            scissors: Vec::new(),
            layers: 0,
            shading: [1, 1, combiner::KEEP, combiner::KEEP],
            rate: None,
            rated: false,
            rendering: false,
            query: None,
            labels: 0,
            natives,
            used: None,
            ring: RING,
            units: Vec::new(),
            fed: None,
        }
    }

    pub fn run(&mut self, mut bytes: &[u8]) -> Result<(), String> {
        self.reset();
        self.shade();
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
            if let Some(r) = &self.extra.rates {
                (r.bind)(0);
            }
            (self.extra.clamp_color)(glow::CLAMP_READ_COLOR, glow::FIXED_ONLY);
            for i in 0..slot::COLORS {
                gl.color_mask_draw_buffer(i, true, true, true, true);
                gl.disable_draw_buffer(glow::BLEND, i);
            }
        }
    }

    fn clip(&self) {
        let (w, h) = self.extent;
        let edge = |a: f32| (a - 0.5).ceil() as i32;
        let boxes: Vec<[i32; 4]> = self
            .views
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let s = self.scissors.get(i).copied().unwrap_or([0, 0, w, h]);
                let (x, y) = (edge(v[0]).max(s[0]), edge(v[1]).max(s[1]));
                let right = edge(v[0] + v[2]).min(s[0] + s[2]);
                let top = edge(v[1] + v[3]).min(s[1] + s[3]);
                [x, y, (right - x).max(0), (top - y).max(0)]
            })
            .collect();
        unsafe {
            self.gl.enable(glow::SCISSOR_TEST);
            match boxes.as_slice() {
                [b] => self.gl.scissor(b[0], b[1], b[2], b[3]),
                _ => self.gl.scissor_slice(0, boxes.len() as i32, &boxes),
            }
        }
    }

    fn shade(&self) {
        let Some(r) = &self.extra.rates else {
            return;
        };
        let on = self.rated || self.shading != [1, 1, combiner::KEEP, combiner::KEEP];
        let palette = rates::palette(self.shading);
        for i in 0..self.caps.viewports.max(1) {
            unsafe {
                if on {
                    (r.palette)(i, 0, rates::ENTRIES as i32, palette.as_ptr());
                    self.gl.enable_draw_buffer(rates::ENABLE, i);
                } else {
                    self.gl.disable_draw_buffer(rates::ENABLE, i);
                }
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
                | Command::ResetQueries { .. } => {}
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
                    if x.min(y).min(z) != 0 && self.prepare(false)? {
                        gl.dispatch_compute(x, y, z);
                    }
                }
                Command::DispatchIndirect { buffer, offset } => {
                    if self.prepare(false)? {
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
                    self.extra.clear.unwrap()(
                        target,
                        glow::R8UI,
                        offset as isize,
                        size as isize,
                        glow::RED_INTEGER,
                        glow::UNSIGNED_BYTE,
                        (&raw const value).cast(),
                    );
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
                    let depth = gl.get_parameter_i32(glow::DEBUG_GROUP_STACK_DEPTH);
                    if depth >= gl.get_parameter_i32(glow::MAX_DEBUG_GROUP_STACK_DEPTH) {
                        return Err("the debug group stack is full".into());
                    }
                    gl.push_debug_group(glow::DEBUG_SOURCE_APPLICATION, 0, text);
                    self.labels += 1;
                }
                Command::EndLabel => {
                    if self.labels > 0 {
                        gl.pop_debug_group();
                        self.labels -= 1;
                    }
                }
                Command::CopyBufferToTexture { .. } => {
                    self.upload(command);
                    self.units.clear();
                }
                Command::CopyTextureToBuffer { .. } => self.readback(command)?,
                Command::CopyTexture { .. } => self.copy_texture(command),
                Command::BlitTexture { .. } => self.blit_texture(command),
                Command::GenerateMips { texture, .. } => {
                    if res.table.textures.get(texture).mips < 2 {
                        return Ok(());
                    }
                    let t = res.textures.get(texture);
                    gl.active_texture(glow::TEXTURE0);
                    gl.bind_texture(t.target, t.raw);
                    gl.generate_mipmap(t.target);
                    gl.bind_texture(t.target, None);
                    self.units.clear();
                }
                Command::ClearTexture { .. } => self.clear_texture(command),
                Command::ColorAttachment {
                    index,
                    texture,
                    layout,
                    load,
                    store,
                    r,
                    g,
                    b,
                    a,
                    resolve,
                    resolve_layout,
                } => {
                    self.colors.retain(|c| c.index != index);
                    self.colors.push(ColorAttachment {
                        index,
                        texture,
                        layout,
                        load,
                        store,
                        r,
                        g,
                        b,
                        a,
                        resolve,
                        resolve_layout,
                    });
                }
                Command::DepthAttachment {
                    texture,
                    layout,
                    load,
                    store,
                    depth,
                    stencil_load,
                    stencil_store,
                    stencil,
                } => {
                    self.depth = Some(DepthAttachment {
                        texture,
                        layout,
                        load,
                        store,
                        depth,
                        stencil_load,
                        stencil_store,
                        stencil,
                    });
                }
                Command::BeginRender {
                    width,
                    height,
                    layers,
                } => self.begin_render(width, height, layers)?,
                Command::EndRender => self.end_render(),
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
                    self.views = vec![[x, y, width, height]];
                    self.clip();
                }
                Command::SetScissor {
                    x,
                    y,
                    width,
                    height,
                } => {
                    self.scissors = vec![[x as i32, y as i32, width as i32, height as i32]];
                    self.clip();
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
                Command::SetShadingRate {
                    width,
                    height,
                    primitive,
                    attachment,
                } => {
                    self.shading = [width, height, primitive, attachment];
                    self.shade();
                }
                Command::RateAttachment {
                    texture,
                    texel_width,
                    texel_height,
                    ..
                } => self.rate = Some((texture, texel_width, texel_height)),
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
                } => {
                    self.vertices[slot as usize] = Some((buffer, offset));
                    self.fed = None;
                }
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
                    self.fed = None;
                }
                Command::Draw {
                    vertices,
                    instances,
                    first_vertex,
                    first_instance,
                } => {
                    if vertices.min(instances) != 0 && self.prepare(true)? {
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
                    if indices.min(instances) != 0 && self.prepare(true)? {
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
                    if count != 0 && self.prepare(true)? {
                        self.indirect(false, buffer, offset, count, stride, None);
                    }
                }
                Command::DrawIndexedIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if count != 0 && self.prepare(true)? {
                        self.indirect(true, buffer, offset, count, stride, None);
                    }
                }
                Command::ExecuteIndirect {
                    indirect,
                    first,
                    count,
                } => {
                    if count != 0 && self.prepare(true)? {
                        let (info, _, mesh) = *res.table.indirects.get(indirect);
                        let stride = crate::table::stride(mesh, info.indexed);
                        let buffer = *res.indirects.get(indirect);
                        let offset = first as u64 * stride as u64;
                        if mesh {
                            self.mesh(buffer, offset, count, stride, None)?;
                        } else {
                            self.indirect(info.indexed, buffer, offset, count, stride, None);
                        }
                    }
                }
                Command::ExecuteIndirectCount {
                    indirect,
                    buffer,
                    offset,
                } => {
                    if self.prepare(true)? {
                        let (info, _, mesh) = *res.table.indirects.get(indirect);
                        let size = info.size;
                        let stride = crate::table::stride(mesh, info.indexed);
                        let commands = *res.indirects.get(indirect);
                        let range = Some((buffer, offset + 4));
                        if mesh {
                            self.mesh(commands, 0, size, stride, range)?;
                        } else {
                            let indexed = info.indexed;
                            self.indirect(indexed, commands, 0, size, stride, range);
                        }
                    }
                }
                Command::DrawMesh { x, y, z } => {
                    if x.min(y).min(z) != 0 && self.prepare(true)? {
                        self.extra.mesh.unwrap()(x, y, z);
                    }
                }
                Command::DrawMeshIndirect {
                    buffer,
                    offset,
                    count,
                    stride,
                } => {
                    if count != 0 && self.prepare(true)? {
                        self.mesh(buffer, offset, count, stride, None)?;
                    }
                }
                Command::WriteTimestamp { heap, index, .. } => {
                    let q = res.queries.get(heap);
                    gl.query_counter(q.ids[index as usize], glow::TIMESTAMP);
                    q.used[index as usize].set(true);
                }
                Command::BeginQuery { heap, index } => {
                    let q = res.queries.get(heap);
                    let target = q.target();
                    gl.begin_query(target, q.ids[index as usize]);
                    q.used[index as usize].set(true);
                    self.query = Some(target);
                }
                Command::EndQuery { heap, .. } => {
                    let target = res.queries.get(heap).target();
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
                        let k = (first + i) as usize;
                        let at = (offset + i as u64 * 8) as usize;
                        if q.used[k].get() {
                            gl.get_query_parameter_u64_with_offset(
                                q.ids[k],
                                glow::QUERY_RESULT,
                                at,
                            );
                        } else {
                            let zero = [0u8; 8];
                            (self.extra.sub)(
                                glow::QUERY_BUFFER,
                                at as isize,
                                8,
                                zero.as_ptr().cast(),
                            );
                        }
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
                    self.views = views
                        .iter()
                        .map(|v| [v.x, v.y, v.width, v.height])
                        .collect();
                    self.clip();
                }
                Command::SetScissors { scissors } => {
                    self.scissors = command::scissors(scissors)
                        .iter()
                        .map(|r| [r.x as i32, r.y as i32, r.width as i32, r.height as i32])
                        .collect();
                    self.clip();
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
                    self.prepare(self.rendering)?;
                    let state = (self.snapshot)();
                    self.natives[index as usize](&Native { gl });
                    state.restore(gl, self.extra);
                    self.fed = None;
                    if self.rendering {
                        self.bind(self.layers)?;
                    }
                }
            }
        }
        Ok(())
    }
}
