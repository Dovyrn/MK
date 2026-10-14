use crate::Error;
use crate::gl::extra::Extra;
use crate::{Caps, register, slot};
use glow::HasContext;
use std::num::NonZeroU32;

const ALPHA_TEST: u32 = 0x0BC0;
const POINT_SPRITE: u32 = 0x8861;
const POINT_SMOOTH: u32 = 0x0B10;
const LINE_STIPPLE: u32 = 0x0B24;
const POLYGON_STIPPLE: u32 = 0x0B42;
const CLAMP_FRAGMENT: u32 = 0x891B;
const COMPATIBILITY: i32 = 0x2;

const TARGETS: [u32; 9] = [
    glow::TEXTURE_1D,
    glow::TEXTURE_2D,
    glow::TEXTURE_3D,
    glow::TEXTURE_1D_ARRAY,
    glow::TEXTURE_2D_ARRAY,
    glow::TEXTURE_CUBE_MAP,
    glow::TEXTURE_CUBE_MAP_ARRAY,
    glow::TEXTURE_2D_MULTISAMPLE,
    glow::TEXTURE_2D_MULTISAMPLE_ARRAY,
];

const ENABLES: [u32; 30] = [
    glow::CULL_FACE,
    glow::SCISSOR_TEST,
    glow::DEPTH_TEST,
    glow::STENCIL_TEST,
    glow::POLYGON_OFFSET_FILL,
    glow::POLYGON_OFFSET_LINE,
    glow::POLYGON_OFFSET_POINT,
    glow::DEPTH_CLAMP,
    glow::SAMPLE_ALPHA_TO_COVERAGE,
    glow::SAMPLE_ALPHA_TO_ONE,
    glow::SAMPLE_COVERAGE,
    glow::SAMPLE_MASK,
    glow::MULTISAMPLE,
    glow::FRAMEBUFFER_SRGB,
    glow::TEXTURE_CUBE_MAP_SEAMLESS,
    glow::PROGRAM_POINT_SIZE,
    glow::RASTERIZER_DISCARD,
    glow::COLOR_LOGIC_OP,
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

pub const PIXELS: [u32; 16] = [
    glow::UNPACK_ALIGNMENT,
    glow::UNPACK_ROW_LENGTH,
    glow::UNPACK_IMAGE_HEIGHT,
    glow::UNPACK_SKIP_PIXELS,
    glow::UNPACK_SKIP_ROWS,
    glow::UNPACK_SKIP_IMAGES,
    glow::UNPACK_SWAP_BYTES,
    glow::UNPACK_LSB_FIRST,
    glow::PACK_ALIGNMENT,
    glow::PACK_ROW_LENGTH,
    glow::PACK_IMAGE_HEIGHT,
    glow::PACK_SKIP_PIXELS,
    glow::PACK_SKIP_ROWS,
    glow::PACK_SKIP_IMAGES,
    glow::PACK_SWAP_BYTES,
    glow::PACK_LSB_FIRST,
];

pub const BLOCKS: [u32; 8] = [
    glow::UNPACK_COMPRESSED_BLOCK_WIDTH,
    glow::UNPACK_COMPRESSED_BLOCK_HEIGHT,
    glow::UNPACK_COMPRESSED_BLOCK_DEPTH,
    glow::UNPACK_COMPRESSED_BLOCK_SIZE,
    glow::PACK_COMPRESSED_BLOCK_WIDTH,
    glow::PACK_COMPRESSED_BLOCK_HEIGHT,
    glow::PACK_COMPRESSED_BLOCK_DEPTH,
    glow::PACK_COMPRESSED_BLOCK_SIZE,
];

const STENCIL: [u32; 14] = [
    glow::STENCIL_FUNC,
    glow::STENCIL_REF,
    glow::STENCIL_VALUE_MASK,
    glow::STENCIL_WRITEMASK,
    glow::STENCIL_FAIL,
    glow::STENCIL_PASS_DEPTH_FAIL,
    glow::STENCIL_PASS_DEPTH_PASS,
    glow::STENCIL_BACK_FUNC,
    glow::STENCIL_BACK_REF,
    glow::STENCIL_BACK_VALUE_MASK,
    glow::STENCIL_BACK_WRITEMASK,
    glow::STENCIL_BACK_FAIL,
    glow::STENCIL_BACK_PASS_DEPTH_FAIL,
    glow::STENCIL_BACK_PASS_DEPTH_PASS,
];

const IMAGES: [u32; 39] = [
    glow::RGBA32F,
    glow::RGBA16F,
    glow::RG32F,
    glow::RG16F,
    glow::R11F_G11F_B10F,
    glow::R32F,
    glow::R16F,
    glow::RGBA32UI,
    glow::RGBA16UI,
    glow::RGB10_A2UI,
    glow::RGBA8UI,
    glow::RG32UI,
    glow::RG16UI,
    glow::RG8UI,
    glow::R32UI,
    glow::R16UI,
    glow::R8UI,
    glow::RGBA32I,
    glow::RGBA16I,
    glow::RGBA8I,
    glow::RG32I,
    glow::RG16I,
    glow::RG8I,
    glow::R32I,
    glow::R16I,
    glow::R8I,
    glow::RGBA16,
    glow::RGB10_A2,
    glow::RGBA8,
    glow::RG16,
    glow::RG8,
    glow::R16,
    glow::R8,
    glow::RGBA16_SNORM,
    glow::RGBA8_SNORM,
    glow::RG16_SNORM,
    glow::RG8_SNORM,
    glow::R16_SNORM,
    glow::R8_SNORM,
];

const BLEND: [u32; 6] = [
    glow::BLEND_SRC_RGB,
    glow::BLEND_DST_RGB,
    glow::BLEND_SRC_ALPHA,
    glow::BLEND_DST_ALPHA,
    glow::BLEND_EQUATION_RGB,
    glow::BLEND_EQUATION_ALPHA,
];

pub fn binding(target: u32) -> u32 {
    match target {
        glow::TEXTURE_1D => glow::TEXTURE_BINDING_1D,
        glow::TEXTURE_2D => glow::TEXTURE_BINDING_2D,
        glow::TEXTURE_3D => glow::TEXTURE_BINDING_3D,
        glow::TEXTURE_1D_ARRAY => glow::TEXTURE_BINDING_1D_ARRAY,
        glow::TEXTURE_2D_ARRAY => glow::TEXTURE_BINDING_2D_ARRAY,
        glow::TEXTURE_CUBE_MAP => glow::TEXTURE_BINDING_CUBE_MAP,
        glow::TEXTURE_CUBE_MAP_ARRAY => glow::TEXTURE_BINDING_CUBE_MAP_ARRAY,
        glow::TEXTURE_2D_MULTISAMPLE => glow::TEXTURE_BINDING_2D_MULTISAMPLE,
        _ => glow::TEXTURE_BINDING_2D_MULTISAMPLE_ARRAY,
    }
}

pub fn buffer(name: i32) -> Option<glow::Buffer> {
    NonZeroU32::new(name as u32).map(glow::NativeBuffer)
}

pub fn set_enabled(gl: &glow::Context, cap: u32, on: bool) {
    unsafe {
        if on {
            gl.enable(cap);
        } else {
            gl.disable(cap);
        }
    }
}

fn texture(name: i32) -> Option<glow::Texture> {
    NonZeroU32::new(name as u32).map(glow::NativeTexture)
}

pub struct State {
    program: i32,
    vao: i32,
    draw: i32,
    read: i32,
    buffers: Vec<(u32, i32)>,
    ranges: Vec<(u32, u32, i32, i64, i64)>,
    images: Vec<[i32; 6]>,
    active: i32,
    units: Vec<(Vec<(u32, i32)>, i32)>,
    targets: Vec<u32>,
    pixels: Vec<(u32, i32)>,
    enables: Vec<(u32, bool)>,
    blends: Vec<bool>,
    masks: Vec<[u8; 4]>,
    funcs: Vec<[i32; 6]>,
    viewports: Vec<[f32; 4]>,
    depths: Vec<[f64; 2]>,
    scissors: Vec<[i32; 4]>,
    tests: Vec<bool>,
    values: [i32; 5],
    polygon: [i32; 2],
    compat: bool,
    origin: i32,
    fragment: i32,
    offset: [f32; 3],
    color: [f32; 4],
    stencil: [i32; 14],
    width: f32,
    write: bool,
    clip: Option<[i32; 2]>,
}

fn unit(gl: &glow::Context, targets: &[u32], u: u32) -> (Vec<(u32, i32)>, i32) {
    unsafe {
        gl.active_texture(glow::TEXTURE0 + u);
        let bound = targets
            .iter()
            .map(|&t| (t, gl.get_parameter_i32(binding(t))))
            .collect();
        (bound, gl.get_parameter_i32(glow::SAMPLER_BINDING))
    }
}

impl State {
    pub fn save_units(&mut self, gl: &glow::Context, count: usize) {
        while self.units.len() < count {
            let u = unit(gl, &self.targets, self.units.len() as u32);
            self.units.push(u);
        }
    }

    pub fn save(
        gl: &glow::Context,
        extra: &Extra,
        caps: &Caps,
        has: &dyn Fn(u32, &str) -> bool,
    ) -> Self {
        unsafe {
            let int = |p| gl.get_parameter_i32(p);
            let indexed = |p, i| {
                let mut v = [0; 4];
                (extra.integers)(p, i, v.as_mut_ptr());
                v
            };
            let mut targets = vec![
                (glow::ARRAY_BUFFER, glow::ARRAY_BUFFER_BINDING),
                (glow::COPY_READ_BUFFER, glow::COPY_READ_BUFFER_BINDING),
                (glow::COPY_WRITE_BUFFER, glow::COPY_WRITE_BUFFER_BINDING),
                (glow::PIXEL_PACK_BUFFER, glow::PIXEL_PACK_BUFFER_BINDING),
                (glow::PIXEL_UNPACK_BUFFER, glow::PIXEL_UNPACK_BUFFER_BINDING),
                (glow::UNIFORM_BUFFER, glow::UNIFORM_BUFFER_BINDING),
            ];
            if extra.indirect.is_some() {
                targets.push((
                    glow::DRAW_INDIRECT_BUFFER,
                    glow::DRAW_INDIRECT_BUFFER_BINDING,
                ));
            }
            if extra.dispatch.is_some() {
                targets.push((
                    glow::DISPATCH_INDIRECT_BUFFER,
                    glow::DISPATCH_INDIRECT_BUFFER_BINDING,
                ));
            }
            if caps.storage_slots {
                targets.push((
                    glow::SHADER_STORAGE_BUFFER,
                    glow::SHADER_STORAGE_BUFFER_BINDING,
                ));
            }
            if has(44, "GL_ARB_query_buffer_object") {
                targets.push((glow::QUERY_BUFFER, glow::QUERY_BUFFER_BINDING));
            }
            let buffers = targets.iter().map(|&(t, b)| (t, int(b))).collect();
            let mut points = vec![(
                glow::UNIFORM_BUFFER,
                [
                    glow::UNIFORM_BUFFER_BINDING,
                    glow::UNIFORM_BUFFER_START,
                    glow::UNIFORM_BUFFER_SIZE,
                ],
                register::UNIFORM + slot::UNIFORMS,
            )];
            if caps.storage_slots {
                points.push((
                    glow::SHADER_STORAGE_BUFFER,
                    [
                        glow::SHADER_STORAGE_BUFFER_BINDING,
                        glow::SHADER_STORAGE_BUFFER_START,
                        glow::SHADER_STORAGE_BUFFER_SIZE,
                    ],
                    slot::STORAGES,
                ));
            }
            let mut ranges = Vec::new();
            for (target, [name, start, size], count) in points {
                for i in 0..count {
                    ranges.push((
                        target,
                        i,
                        indexed(name, i)[0],
                        gl.get_parameter_indexed_i64(start, i),
                        gl.get_parameter_indexed_i64(size, i),
                    ));
                }
            }
            let images = if has(42, "GL_ARB_shader_image_load_store") {
                (0..slot::IMAGES)
                    .map(|i| {
                        [
                            glow::IMAGE_BINDING_NAME,
                            glow::IMAGE_BINDING_LEVEL,
                            glow::IMAGE_BINDING_LAYERED,
                            glow::IMAGE_BINDING_LAYER,
                            glow::IMAGE_BINDING_ACCESS,
                            glow::IMAGE_BINDING_FORMAT,
                        ]
                        .map(|p| indexed(p, i)[0])
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let active = int(glow::ACTIVE_TEXTURE);
            let targets: Vec<u32> = TARGETS
                .into_iter()
                .filter(|&t| {
                    t != glow::TEXTURE_CUBE_MAP_ARRAY || has(40, "GL_ARB_texture_cube_map_array")
                })
                .collect();
            let mut pixels: Vec<u32> = PIXELS.to_vec();
            if has(42, "GL_ARB_compressed_texture_pixel_storage") {
                pixels.extend(BLOCKS);
            }
            let pixels = pixels.into_iter().map(|p| (p, int(p))).collect();
            let compat = int(glow::CONTEXT_PROFILE_MASK) & COMPATIBILITY != 0;
            let mut enables = ENABLES.to_vec();
            if has(43, "GL_ARB_ES3_compatibility") {
                enables.push(glow::PRIMITIVE_RESTART_FIXED_INDEX);
            }
            if has(40, "GL_ARB_sample_shading") {
                enables.push(glow::SAMPLE_SHADING);
            }
            if compat {
                enables.extend([
                    ALPHA_TEST,
                    POINT_SPRITE,
                    POINT_SMOOTH,
                    LINE_STIPPLE,
                    POLYGON_STIPPLE,
                ]);
            }
            let enables = enables.into_iter().map(|e| (e, gl.is_enabled(e))).collect();
            let blends = (0..slot::COLORS)
                .map(|i| (extra.enabled)(glow::BLEND, i) != 0)
                .collect();
            let masks = (0..slot::COLORS)
                .map(|i| {
                    let mut m = [0u8; 4];
                    (extra.booleans)(glow::COLOR_WRITEMASK, i, m.as_mut_ptr());
                    m
                })
                .collect();
            let funcs = if caps.independent_blend {
                (0..slot::COLORS)
                    .map(|i| BLEND.map(|p| indexed(p, i)[0]))
                    .collect()
            } else {
                vec![BLEND.map(int)]
            };
            let (mut viewports, mut depths, mut scissors) = (Vec::new(), Vec::new(), Vec::new());
            let mut tests = Vec::new();
            match (extra.floats, extra.doubles) {
                (Some(floats), Some(doubles)) => {
                    for i in 0..int(glow::MAX_VIEWPORTS) as u32 {
                        let mut v = [0.0; 4];
                        floats(glow::VIEWPORT, i, v.as_mut_ptr());
                        viewports.push(v);
                        let mut d = [0.0; 2];
                        doubles(glow::DEPTH_RANGE, i, d.as_mut_ptr());
                        depths.push(d);
                        scissors.push(indexed(glow::SCISSOR_BOX, i));
                        tests.push((extra.enabled)(glow::SCISSOR_TEST, i) != 0);
                    }
                }
                _ => {
                    let mut v = [0; 4];
                    gl.get_parameter_i32_slice(glow::VIEWPORT, &mut v);
                    viewports.push(v.map(|x| x as f32));
                    let mut d = [0.0; 2];
                    (extra.double)(glow::DEPTH_RANGE, d.as_mut_ptr());
                    depths.push(d);
                    let mut s = [0; 4];
                    gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut s);
                    scissors.push(s);
                }
            }
            let mut polygon = [0; 2];
            gl.get_parameter_i32_slice(glow::POLYGON_MODE, &mut polygon);
            let mut color = [0.0; 4];
            gl.get_parameter_f32_slice(glow::BLEND_COLOR, &mut color);
            let clamp = if extra.offset_clamp.is_some() {
                gl.get_parameter_f32(glow::POLYGON_OFFSET_CLAMP)
            } else {
                0.0
            };
            Self {
                program: int(glow::CURRENT_PROGRAM),
                vao: int(glow::VERTEX_ARRAY_BINDING),
                draw: int(glow::DRAW_FRAMEBUFFER_BINDING),
                read: int(glow::READ_FRAMEBUFFER_BINDING),
                buffers,
                ranges,
                images,
                active,
                units: Vec::new(),
                targets,
                pixels,
                enables,
                blends,
                masks,
                funcs,
                viewports,
                depths,
                scissors,
                tests,
                values: [
                    glow::CULL_FACE_MODE,
                    glow::FRONT_FACE,
                    glow::PROVOKING_VERTEX,
                    glow::DEPTH_FUNC,
                    glow::CLAMP_READ_COLOR,
                ]
                .map(int),
                polygon,
                compat,
                origin: int(glow::POINT_SPRITE_COORD_ORIGIN),
                fragment: if compat { int(CLAMP_FRAGMENT) } else { 0 },
                offset: [
                    gl.get_parameter_f32(glow::POLYGON_OFFSET_FACTOR),
                    gl.get_parameter_f32(glow::POLYGON_OFFSET_UNITS),
                    clamp,
                ],
                color,
                stencil: STENCIL.map(int),
                width: gl.get_parameter_f32(glow::LINE_WIDTH),
                write: gl.get_parameter_bool(glow::DEPTH_WRITEMASK),
                clip: extra
                    .clip
                    .map(|_| [glow::CLIP_ORIGIN, glow::CLIP_DEPTH_MODE].map(int)),
            }
        }
    }

    pub fn clear(&self, gl: &glow::Context, extra: &Extra) {
        for &(p, _) in &self.pixels {
            let aligned = p == glow::UNPACK_ALIGNMENT || p == glow::PACK_ALIGNMENT;
            unsafe { gl.pixel_store_i32(p, aligned as i32) };
        }
        if let Some(clip) = extra.clip {
            unsafe { clip(glow::LOWER_LEFT, glow::NEGATIVE_ONE_TO_ONE) };
        }
        for &(e, _) in &self.enables {
            if matches!(
                e,
                glow::PRIMITIVE_RESTART_FIXED_INDEX
                    | glow::SAMPLE_SHADING
                    | ALPHA_TEST
                    | POINT_SMOOTH
                    | LINE_STIPPLE
                    | POLYGON_STIPPLE
            ) {
                unsafe { gl.disable(e) };
            }
            if e == POINT_SPRITE {
                unsafe { gl.enable(e) };
            }
        }
        unsafe { (extra.point)(glow::POINT_SPRITE_COORD_ORIGIN, glow::UPPER_LEFT as i32) };
        if self.compat {
            unsafe { (extra.clamp_color)(CLAMP_FRAGMENT, glow::FALSE as u32) };
        }
    }

    pub fn restore(&self, gl: &glow::Context, extra: &Extra) -> Result<(), Error> {
        let check = |what: &str| {
            let errors = if cfg!(debug_assertions) {
                crate::gl::errors::drain(gl)
            } else {
                Vec::new()
            };
            if errors.is_empty() {
                Ok(())
            } else {
                Err(Error::Driver(format!(
                    "GL errors {errors:#x?} restoring host {what}"
                )))
            }
        };
        unsafe {
            gl.use_program(NonZeroU32::new(self.program as u32).map(glow::NativeProgram));
            gl.bind_vertex_array(NonZeroU32::new(self.vao as u32).map(glow::NativeVertexArray));
            let framebuffer = |name: i32| NonZeroU32::new(name as u32).map(glow::NativeFramebuffer);
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, framebuffer(self.draw));
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, framebuffer(self.read));
            for &(target, i, name, start, size) in &self.ranges {
                if name != 0 && size != 0 {
                    (extra.range)(target, i, name as u32, start as isize, size as isize);
                } else {
                    gl.bind_buffer_base(target, i, buffer(name));
                }
            }
            check("framebuffers and buffer ranges")?;
            for &(target, name) in &self.buffers {
                gl.bind_buffer(target, buffer(name));
            }
            check("buffer bindings")?;
            for (i, image) in self.images.iter().enumerate() {
                let [name, level, layered, layer, access, format] = *image;
                if !IMAGES.contains(&(format as u32)) {
                    extra.images.unwrap()(i as u32, 1, &(name as u32));
                    continue;
                }
                gl.bind_image_texture(
                    i as u32,
                    texture(name),
                    level,
                    layered != 0,
                    layer,
                    access as u32,
                    format as u32,
                );
            }
            check("image units")?;
            for (u, (bound, sampler)) in self.units.iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + u as u32);
                for &(target, name) in bound {
                    gl.bind_texture(target, texture(name));
                }
                let sampler = NonZeroU32::new(*sampler as u32).map(glow::NativeSampler);
                gl.bind_sampler(u as u32, sampler);
            }
            check("texture units")?;
            gl.active_texture(self.active as u32);
            for &(p, v) in &self.pixels {
                gl.pixel_store_i32(p, v);
            }
            check("pixel store")?;
            for &(e, on) in &self.enables {
                set_enabled(gl, e, on);
            }
            check("enables")?;
            for (i, &on) in self.blends.iter().enumerate() {
                if on {
                    gl.enable_draw_buffer(glow::BLEND, i as u32);
                } else {
                    gl.disable_draw_buffer(glow::BLEND, i as u32);
                }
            }
            for (i, m) in self.masks.iter().enumerate() {
                let [r, g, b, a] = m.map(|c| c != 0);
                gl.color_mask_draw_buffer(i as u32, r, g, b, a);
            }
            match self.funcs.as_slice() {
                [f] => {
                    let f = f.map(|v| v as u32);
                    gl.blend_func_separate(f[0], f[1], f[2], f[3]);
                    if f[4] == f[5] {
                        gl.blend_equation(f[4]);
                    } else {
                        gl.blend_equation_separate(f[4], f[5]);
                    }
                }
                funcs => {
                    for (i, f) in funcs.iter().enumerate() {
                        let f = f.map(|v| v as u32);
                        let i = i as u32;
                        gl.blend_func_separate_draw_buffer(i, f[0], f[1], f[2], f[3]);
                        if f[4] == f[5] {
                            gl.blend_equation_draw_buffer(i, f[4]);
                        } else {
                            gl.blend_equation_separate_draw_buffer(i, f[4], f[5]);
                        }
                    }
                }
            }
            check("blending and masks")?;
            match (self.viewports.as_slice(), self.depths.as_slice()) {
                ([v], [d]) => {
                    gl.viewport(v[0] as i32, v[1] as i32, v[2] as i32, v[3] as i32);
                    gl.depth_range(d[0], d[1]);
                    let s = self.scissors[0];
                    gl.scissor(s[0], s[1], s[2], s[3]);
                }
                _ => {
                    let count = self.viewports.len() as i32;
                    gl.viewport_f32_slice(0, count, &self.viewports);
                    gl.depth_range_f64_slice(0, count, &self.depths);
                    gl.scissor_slice(0, count, &self.scissors);
                    for (i, &on) in self.tests.iter().enumerate() {
                        if on {
                            gl.enable_draw_buffer(glow::SCISSOR_TEST, i as u32);
                        } else {
                            gl.disable_draw_buffer(glow::SCISSOR_TEST, i as u32);
                        }
                    }
                }
            }
            check("viewports and scissors")?;
            let [cull, front, provoking, func, clamp] = self.values.map(|v| v as u32);
            gl.cull_face(cull);
            gl.front_face(front);
            (extra.provoking)(provoking);
            gl.depth_func(func);
            gl.depth_mask(self.write);
            (extra.clamp_color)(glow::CLAMP_READ_COLOR, clamp);
            let [front, back] = self.polygon.map(|p| p as u32);
            if !self.compat || front == back {
                gl.polygon_mode(glow::FRONT_AND_BACK, front);
            } else {
                gl.polygon_mode(glow::FRONT, front);
                gl.polygon_mode(glow::BACK, back);
            }
            (extra.point)(glow::POINT_SPRITE_COORD_ORIGIN, self.origin);
            if self.compat {
                (extra.clamp_color)(CLAMP_FRAGMENT, self.fragment as u32);
            }
            check("raster state")?;
            let [factor, units, limit] = self.offset;
            match extra.offset_clamp {
                Some(offset) => offset(factor, units, limit),
                None => gl.polygon_offset(factor, units),
            }
            let [r, g, b, a] = self.color;
            gl.blend_color(r, g, b, a);
            let s = self.stencil;
            for (face, k) in [(glow::FRONT, 0), (glow::BACK, 7)] {
                gl.stencil_func_separate(face, s[k] as u32, s[k + 1], s[k + 2] as u32);
                gl.stencil_mask_separate(face, s[k + 3] as u32);
                gl.stencil_op_separate(face, s[k + 4] as u32, s[k + 5] as u32, s[k + 6] as u32);
            }
            gl.line_width(self.width);
            if let (Some(clip), Some([origin, depth])) = (extra.clip, self.clip) {
                clip(origin as u32, depth as u32);
            }
            check("stencil, line width and clip")
        }
    }
}
