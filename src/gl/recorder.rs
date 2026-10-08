use crate::command::Command;
mod copy;
mod draw;
mod pass;
mod textures;

use crate::caps::same_blend;
use crate::gl::device::compare;
use crate::gl::extra::Extra;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::resources::Resources;
use crate::gl::state::{State, set_enabled};
use crate::register;
use crate::{Buffer, Sampler, Texture};
use crate::{
    Caps, DepthAttachment, DepthStateInfo, cull_mode, format, front_face, index_type, slot, stage,
    topology,
};
use glow::HasContext;
use std::cell::Cell;

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

#[derive(Clone, Copy)]
pub struct Color {
    pub index: u32,
    pub texture: Texture,
    pub load: u32,
    pub store: u32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
    pub resolve: Texture,
}

pub struct Recorder<'a> {
    gl: &'a glow::Context,
    extra: &'a Extra,
    res: &'a Resources,
    caps: &'a Caps,
    fbo: glow::Framebuffer,
    read: glow::Framebuffer,
    program: Option<&'a Program>,
    render: Option<&'a Pipeline>,
    colors: Vec<Color>,
    depth: Option<DepthAttachment>,
    textures: [Option<Texture>; slot::TEXTURES as usize],
    samplers: [Option<Sampler>; slot::SAMPLERS as usize],
    vertices: [Option<(Buffer, u64)>; slot::VERTICES as usize],
    index: Option<(Buffer, u64, u32)>,
    mode: u32,
    state: Option<DepthStateInfo>,
    references: (u32, u32),
    extent: (i32, i32),
    views: Vec<[f32; 4]>,
    scissors: Vec<[i32; 4]>,
    layers: u32,
    rendering: bool,
    query: Option<u32>,
    labels: u32,
    host: Option<State>,
    open: &'a Cell<bool>,
    pub imported: bool,
    used: Option<glow::Program>,
    units: Vec<Option<(Option<Texture>, Option<Sampler>)>>,
    fed: Option<glow::VertexArray>,
}

fn gl_primitive_mode(t: u32) -> u32 {
    match t {
        topology::POINT => glow::POINTS,
        topology::LINE => glow::LINES,
        topology::LINE_STRIP => glow::LINE_STRIP,
        topology::TRIANGLE => glow::TRIANGLES,
        topology::TRIANGLE_STRIP => glow::TRIANGLE_STRIP,
        t => panic!("unknown topology {t}"),
    }
}

fn gl_stencil_op(op: u32) -> u32 {
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

fn gl_blend_factor(f: u32) -> u32 {
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

fn gl_blend_equation(op: u32) -> u32 {
    [
        glow::FUNC_ADD,
        glow::FUNC_SUBTRACT,
        glow::FUNC_REVERSE_SUBTRACT,
        glow::MIN,
        glow::MAX,
    ][op as usize]
}

fn is_integer_format(f: u32) -> bool {
    format::uint(f) || format::sint(f)
}

fn gl_barrier_bits(stages: u32) -> u32 {
    let shader = glow::UNIFORM_BARRIER_BIT
        | glow::SHADER_STORAGE_BARRIER_BIT
        | glow::SHADER_IMAGE_ACCESS_BARRIER_BIT
        | glow::TEXTURE_FETCH_BARRIER_BIT;
    let map = [
        (stage::INDIRECT, glow::COMMAND_BARRIER_BIT),
        (
            stage::VERTEX,
            shader | glow::ELEMENT_ARRAY_BARRIER_BIT | glow::VERTEX_ATTRIB_ARRAY_BARRIER_BIT,
        ),
        (
            stage::FRAGMENT | stage::TASK | stage::MESH | stage::COMPUTE,
            shader,
        ),
        (stage::DEPTH | stage::COLOR, glow::FRAMEBUFFER_BARRIER_BIT),
        (
            stage::COPY,
            glow::BUFFER_UPDATE_BARRIER_BIT
                | glow::TEXTURE_UPDATE_BARRIER_BIT
                | glow::PIXEL_BUFFER_BARRIER_BIT
                | glow::FRAMEBUFFER_BARRIER_BIT,
        ),
        (
            stage::HOST,
            glow::BUFFER_UPDATE_BARRIER_BIT
                | glow::PIXEL_BUFFER_BARRIER_BIT
                | glow::CLIENT_MAPPED_BUFFER_BARRIER_BIT,
        ),
    ];
    map.iter()
        .filter(|(b, _)| stages & b != 0)
        .fold(0, |m, (_, g)| m | g)
}

impl<'a> Recorder<'a> {
    pub fn new(
        gl: &'a glow::Context,
        extra: &'a Extra,
        res: &'a Resources,
        caps: &'a Caps,
        (fbo, read): (glow::Framebuffer, glow::Framebuffer),
        host: State,
        open: &'a Cell<bool>,
    ) -> Self {
        Self {
            gl,
            extra,
            res,
            caps,
            fbo,
            read,
            program: None,
            render: None,
            colors: Vec::new(),
            depth: None,
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
            rendering: false,
            query: None,
            labels: 0,
            host: Some(host),
            open,
            imported: false,
            used: None,
            units: Vec::new(),
            fed: None,
        }
    }

    pub fn begin(&mut self) -> Result<(), String> {
        self.reset_state();
        self.check_errors("begin_commands")
    }

    pub fn import(&mut self, width: u32, height: u32) {
        self.imported = true;
        self.rendering = true;
        self.extent = (width as i32, height as i32);
        self.views = vec![[0.0, 0.0, width as f32, height as f32]];
        self.layers = 1;
    }

    pub fn step(&mut self, command: Command) -> Result<(), String> {
        if !cfg!(debug_assertions) {
            return self.apply(command);
        }
        let name = command.to_string();
        self.apply(command)?;
        self.check_errors(&name)
    }

    fn check_errors(&self, what: &str) -> Result<(), String> {
        if !cfg!(debug_assertions) {
            return Ok(());
        }
        let errors = crate::gl::errors::drain(self.gl);
        if errors.is_empty() {
            return Ok(());
        }
        Err(format!("GL errors {errors:#x?} in {what}"))
    }

    pub fn finish(&mut self) -> Result<(), String> {
        let Some(host) = self.host.take() else {
            return Ok(());
        };
        self.open.set(false);
        let gl = self.gl;
        unsafe {
            if let Some(target) = self.query.take() {
                gl.end_query(target);
            }
            for _ in 0..self.labels {
                gl.pop_debug_group();
            }
        }
        self.labels = 0;
        let mine = crate::gl::errors::drain(gl);
        host.restore(gl, self.extra)?;
        if mine.is_empty() {
            Ok(())
        } else {
            Err(format!("recording raised GL errors {mine:#x?}"))
        }
    }

    fn bind_range(&self, target: u32, index: u32, buffer: Buffer, offset: u64, size: u64) {
        let b = self.res.buffers.get(buffer);
        let size = if size == !0 { b.size - offset } else { size };
        unsafe { (self.extra.range)(target, index, b.raw.0.get(), offset as isize, size as isize) };
    }

    fn apply_blend_state(&self, p: &Pipeline) {
        let gl = self.gl;
        let uniform = same_blend(&p.colors);
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
                let (src, dst) = (gl_blend_factor(c.src), gl_blend_factor(c.dst));
                let (src_alpha, dst_alpha) =
                    (gl_blend_factor(c.src_alpha), gl_blend_factor(c.dst_alpha));
                let (op, op_alpha) = (gl_blend_equation(c.op), gl_blend_equation(c.op_alpha));
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

    fn reset_state(&mut self) {
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

    fn apply_scissors(&self) {
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

    fn apply_stencil(&self) {
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
                    gl_stencil_op(f.fail),
                    gl_stencil_op(f.depth_fail),
                    gl_stencil_op(f.pass),
                );
                gl.stencil_func_separate(side, compare(f.compare), reference as i32, f.read);
                gl.stencil_mask_separate(side, f.write);
            }
        }
    }

    fn apply(&mut self, command: Command) -> Result<(), String> {
        let gl = self.gl;
        let res = self.res;
        unsafe {
            match command {
                Command::BeginComputePass
                | Command::EndComputePass
                | Command::OcclusionPool { .. }
                | Command::ResetQueries { .. } => {}
                Command::BindComputePipeline { pipeline } => {
                    let p = res.pipelines.get(pipeline);
                    self.program = Some(p);
                }
                Command::Dispatch { x, y, z } => {
                    if x.min(y).min(z) != 0 && self.bind_draw_state(false)? {
                        gl.dispatch_compute(x, y, z);
                    }
                }
                Command::DispatchIndirect { buffer, offset } => {
                    if self.bind_draw_state(false)? {
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
                Command::MemoryBarrier {
                    src_stage,
                    dst_stage,
                }
                | Command::TextureBarrier {
                    src_stage,
                    dst_stage,
                    ..
                } => {
                    let shaders = stage::VERTEX
                        | stage::FRAGMENT
                        | stage::TASK
                        | stage::MESH
                        | stage::COMPUTE;
                    if src_stage & shaders != 0 && self.caps.storage {
                        gl.memory_barrier(gl_barrier_bits(dst_stage));
                    }
                }
                Command::BeginDebugLabel { text } => {
                    if !self.extra.debug {
                        return Ok(());
                    }
                    let full = || {
                        let depth = gl.get_parameter_i32(glow::DEBUG_GROUP_STACK_DEPTH);
                        depth >= gl.get_parameter_i32(glow::MAX_DEBUG_GROUP_STACK_DEPTH)
                    };
                    if cfg!(feature = "validation") && full() {
                        return Err("the debug group stack is full".into());
                    }
                    gl.push_debug_group(glow::DEBUG_SOURCE_APPLICATION, 0, text);
                    self.labels += 1;
                }
                Command::EndDebugLabel => {
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
                        r,
                        g,
                        b,
                        a,
                        resolve,
                    });
                }
                Command::DepthAttachment {
                    texture,
                    load,
                    store,
                    depth,
                    stencil_load,
                    stencil_store,
                    stencil,
                } => {
                    self.depth = Some(DepthAttachment {
                        texture,
                        load,
                        store,
                        depth,
                        stencil_load,
                        stencil_store,
                        stencil,
                    });
                }
                Command::BeginRenderPass {
                    width,
                    height,
                    layers,
                } => {
                    self.begin_render(width, height, layers)?;
                    gl.viewport(0, 0, width as i32, height as i32);
                    gl.depth_range(0.0, 1.0);
                    self.views = vec![[0.0, 0.0, width as f32, height as f32]];
                    self.scissors = vec![[0, 0, width as i32, height as i32]];
                    self.apply_scissors();
                }
                Command::EndRenderPass => self.end_render()?,
                Command::InputAttachmentBarrier
                | Command::TileMemory { .. }
                | Command::DispatchTile { .. } => {
                    unreachable!("caps.attachment_reads and caps.tiles are false on GL")
                }
                Command::BindRenderPipeline { pipeline } => {
                    let p = res.renders.get(pipeline);
                    self.render = Some(p);
                    self.mode = gl_primitive_mode(p.topology);
                    let fill = if p.fill == crate::fill_mode::LINE {
                        glow::LINE
                    } else {
                        glow::FILL
                    };
                    gl.polygon_mode(glow::FRONT_AND_BACK, fill);
                    set_enabled(gl, glow::DEPTH_CLAMP, p.clamp);
                    set_enabled(gl, glow::SAMPLE_ALPHA_TO_COVERAGE, p.coverage);
                    self.apply_blend_state(p);
                }
                Command::SetCullMode { cull: c } => match c {
                    cull_mode::NONE => gl.disable(glow::CULL_FACE),
                    cull_mode::FRONT | cull_mode::BACK => {
                        gl.enable(glow::CULL_FACE);
                        gl.cull_face(if c == cull_mode::FRONT {
                            glow::FRONT
                        } else {
                            glow::BACK
                        });
                    }
                    c => panic!("unknown cull {c}"),
                },
                Command::SetFrontFace { face: f } => gl.front_face(match f {
                    front_face::CCW => glow::CW,
                    front_face::CW => glow::CCW,
                    f => panic!("unknown face {f}"),
                }),
                Command::SetPrimitiveTopology { topology } => {
                    self.mode = gl_primitive_mode(topology)
                }
                Command::SetDepthState { state } => {
                    let d = *res.depths.get(state);
                    set_enabled(
                        gl,
                        glow::DEPTH_TEST,
                        d.compare != crate::compare_op::NONE || d.write,
                    );
                    gl.depth_func(compare(d.compare));
                    gl.depth_mask(d.write);
                    set_enabled(gl, glow::STENCIL_TEST, d.stencil);
                    self.state = Some(d);
                    self.apply_stencil();
                }
                Command::SetStencilReference { front, back } => {
                    self.references = (front, back);
                    self.apply_stencil();
                }
                Command::SetDepthBias {
                    constant,
                    slope,
                    clamp,
                } => {
                    let on = constant != 0.0 || slope != 0.0;
                    for mode in OFFSETS {
                        set_enabled(gl, mode, on);
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
                        index_type::U16 => glow::UNSIGNED_SHORT,
                        index_type::U32 => glow::UNSIGNED_INT,
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
                    if vertices.min(instances) != 0 && self.bind_draw_state(true)? {
                        self.draw_direct(
                            false,
                            vertices,
                            instances,
                            first_vertex,
                            0,
                            first_instance,
                        );
                    }
                }
                Command::DrawIndexed {
                    indices,
                    instances,
                    first_index,
                    base_vertex,
                    first_instance,
                } => {
                    if indices.min(instances) != 0 && self.bind_draw_state(true)? {
                        self.draw_direct(
                            true,
                            indices,
                            instances,
                            first_index,
                            base_vertex,
                            first_instance,
                        );
                    }
                }
                Command::DrawIndirect { buffer, offset } => {
                    if self.bind_draw_state(true)? {
                        self.draw_indirect_buffer(false, buffer, offset);
                    }
                }
                Command::DrawIndexedIndirect { buffer, offset } => {
                    if self.bind_draw_state(true)? {
                        self.draw_indirect_buffer(true, buffer, offset);
                    }
                }
                Command::DrawMeshTasks { x, y, z } => {
                    if x.min(y).min(z) != 0 && self.bind_draw_state(true)? {
                        self.extra.mesh.unwrap()(x, y, z);
                    }
                }
                Command::ExecuteIndirect { .. } | Command::ExecuteIndirectCount { .. } => {
                    unreachable!("create_indirect_buffer needs caps.addresses, which GL lacks")
                }
                Command::DrawMeshTasksIndirect { buffer, offset } => {
                    if self.bind_draw_state(true)? {
                        self.draw_mesh_tasks(buffer, offset);
                    }
                }
                Command::WriteTimestamp {
                    pool: heap, index, ..
                } => {
                    let q = res.queries.get(heap);
                    gl.query_counter(q.ids[index as usize], glow::TIMESTAMP);
                    q.used[index as usize].set(true);
                }
                Command::BeginOcclusionQuery { pool: heap, index } => {
                    let q = res.queries.get(heap);
                    let target = q.target();
                    gl.begin_query(target, q.ids[index as usize]);
                    q.used[index as usize].set(true);
                    self.query = Some(target);
                }
                Command::EndOcclusionQuery { pool: heap, .. } => {
                    let target = res.queries.get(heap).target();
                    if self.query.take() == Some(target) {
                        gl.end_query(target);
                    }
                }
                Command::ResolveTimestamps {
                    pool: heap,
                    first,
                    count,
                    buffer,
                    offset,
                }
                | Command::ResolveOcclusion {
                    pool: heap,
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
                Command::SetViewport { viewports } => {
                    let views: Vec<_> = viewports.iter().collect();
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
                    self.apply_scissors();
                }
                Command::SetScissor { scissors } => {
                    self.scissors = scissors
                        .iter()
                        .map(|r| [r.x as i32, r.y as i32, r.width as i32, r.height as i32])
                        .collect();
                    self.apply_scissors();
                }
                Command::UseResidencySet { .. } => {}
                Command::BindUniformBuffer {
                    slot,
                    buffer,
                    offset,
                    size,
                } => self.bind_range(
                    glow::UNIFORM_BUFFER,
                    register::UNIFORM + slot,
                    buffer,
                    offset,
                    size,
                ),
                Command::BindStorageBuffer {
                    slot,
                    buffer,
                    offset,
                    size,
                } => {
                    self.bind_range(glow::SHADER_STORAGE_BUFFER, slot, buffer, offset, size);
                }
                Command::BindStorageTexture { slot, texture } => {
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
            }
        }
        Ok(())
    }
}

impl Drop for Recorder<'_> {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
