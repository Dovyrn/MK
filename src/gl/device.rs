use crate::caps;
use crate::check::Check;
use crate::gl::buffer::Buffer;
use crate::gl::cache::Cache;
use crate::gl::event::Event;
use crate::gl::extra::Extra;
use crate::gl::native::Callback;
use crate::gl::pipeline::{Pipeline, uniform};
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::recorder::Recorder;
use crate::gl::resolve::Resolve;
use crate::gl::resources::Resources;
use crate::gl::state::{self, State, binding, buffer};
use crate::gl::texture::Texture;
use crate::{
    Caps, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, SamplerInfo, TextureInfo, ViewInfo,
    address, border, dimension, filter, format, memory, query, usage,
};
use glow::HasContext;
use std::cell::Cell;
use std::ffi::{c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

pub struct Device {
    pub gl: glow::Context,
    extra: Extra,
    version: u32,
    extensions: Vec<String>,
    res: Resources,
    push: glow::Buffer,
    fbo: glow::Framebuffer,
    read: glow::Framebuffer,
    resolve: Resolve,
    cache: Option<Cache>,
    units: u32,
    caps: Caps,
    anisotropy: f32,
    samples: [i32; 4],
}

const MAX_ANISOTROPY: u32 = 0x84FE;

const MAX_ANISOTROPY_LIMIT: u32 = 0x84FF;

const TASK_SHADER: u32 = 0x955A;

const MESH_SHADER: u32 = 0x9559;

pub fn compare(c: u32) -> u32 {
    match c {
        crate::compare::NONE => glow::ALWAYS,
        crate::compare::NEVER..=crate::compare::ALWAYS => glow::NEVER + c - 1,
        c => panic!("unknown compare {c}"),
    }
}

pub fn target(dimension: u32, samples: u32) -> u32 {
    match (dimension, samples > 1) {
        (dimension::D1, _) => glow::TEXTURE_1D,
        (dimension::D1_ARRAY, _) => glow::TEXTURE_1D_ARRAY,
        (dimension::D2, false) => glow::TEXTURE_2D,
        (dimension::D2, true) => glow::TEXTURE_2D_MULTISAMPLE,
        (dimension::D2_ARRAY, false) => glow::TEXTURE_2D_ARRAY,
        (dimension::D2_ARRAY, true) => glow::TEXTURE_2D_MULTISAMPLE_ARRAY,
        (dimension::D3, _) => glow::TEXTURE_3D,
        (dimension::CUBE, _) => glow::TEXTURE_CUBE_MAP,
        (dimension::CUBE_ARRAY, _) => glow::TEXTURE_CUBE_MAP_ARRAY,
        (d, _) => panic!("unknown dimension {d}"),
    }
}

unsafe fn drain(gl: &glow::Context) {
    while !matches!(
        unsafe { gl.get_error() },
        glow::NO_ERROR | glow::CONTEXT_LOST
    ) {}
}

fn has(version: u32, extensions: &[String], minimum: u32, extension: &str) -> bool {
    version >= minimum
        || extensions
            .binary_search_by(|e| e.as_str().cmp(extension))
            .is_ok()
}

impl Device {
    pub fn adopt(proc: impl Fn(&str) -> *const c_void) -> Result<Self, String> {
        Self::load(&proc)
    }

    pub fn adopt_raw(proc: u64) -> Result<Self, String> {
        let get = unsafe {
            std::mem::transmute::<usize, unsafe extern "C" fn(*const c_char) -> *const c_void>(
                proc as usize,
            )
        };
        Self::load(&|name| {
            let name = std::ffi::CString::new(name).unwrap();
            unsafe { get(name.as_ptr()) }
        })
    }

    fn load(proc: &dyn Fn(&str) -> *const c_void) -> Result<Self, String> {
        unsafe {
            let gl = glow::Context::from_loader_function(|name| proc(name));
            let major = gl.get_parameter_i32(glow::MAJOR_VERSION) as u32;
            let minor = gl.get_parameter_i32(glow::MINOR_VERSION) as u32;
            let version = major * 10 + minor;
            if version < 33 {
                return Err(format!("GL {major}.{minor} is below 3.3"));
            }
            let count = gl.get_parameter_i32(glow::NUM_EXTENSIONS) as u32;
            let mut extensions: Vec<String> = (0..count)
                .map(|i| gl.get_parameter_indexed_string(glow::EXTENSIONS, i))
                .collect();
            extensions.sort();
            let has = |v, e: &str| has(version, &extensions, v, e);
            let extra = Extra::load(proc, &has)?;
            let int = |p| gl.get_parameter_i32(p);
            let storage = has(43, "GL_ARB_shader_storage_buffer_object");
            let results = has(44, "GL_ARB_query_buffer_object");
            let caps = Caps {
                push: 256,
                queues: 1,
                tick: 1.0,
                timestamps: results,
                precise: results,
                viewports: if has(99, "GL_ARB_shader_viewport_layer_array") {
                    int(glow::MAX_VIEWPORTS) as u32
                } else {
                    1
                },
                graphics: 1,
                labels: has(43, "GL_KHR_debug"),
                anisotropy: has(46, "GL_ARB_texture_filter_anisotropic")
                    || has(99, "GL_EXT_texture_filter_anisotropic"),
                depth_clamp: true,
                line_fill: true,
                multi_draw: has(43, "GL_ARB_multi_draw_indirect"),
                slots: true,
                storage,
                persistent: has(44, "GL_ARB_buffer_storage"),
                bind_alignment: if storage {
                    int(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
                        .max(int(glow::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT))
                        as u32
                } else {
                    int(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT) as u32
                },
                compute: has(43, "GL_ARB_compute_shader"),
                texture_views: has(43, "GL_ARB_texture_view"),
                occlusion: results,
                cache: int(glow::NUM_PROGRAM_BINARY_FORMATS) > 0,
                bias_clamp: has(46, "GL_ARB_polygon_offset_clamp"),
                blends: has(40, "GL_ARB_draw_buffers_blend"),
                indirect: has(42, "GL_ARB_draw_indirect") && has(42, "GL_ARB_base_instance"),
                indirect_ids: false,
                indirect_offset: false,
                wide_lines: false,
                max_buffer: isize::MAX as u64,
                max_texture: int(glow::MAX_TEXTURE_SIZE) as u32,
                max_cube: int(glow::MAX_CUBE_MAP_TEXTURE_SIZE) as u32,
                max_volume: int(glow::MAX_3D_TEXTURE_SIZE) as u32,
                max_layers: int(glow::MAX_ARRAY_TEXTURE_LAYERS) as u32,
                mesh: extra.mesh.is_some(),
                ..Default::default()
            };
            let anisotropy = if caps.anisotropy {
                gl.get_parameter_f32(MAX_ANISOTROPY_LIMIT)
            } else {
                1.0
            };
            let samples = [
                int(glow::MAX_DEPTH_TEXTURE_SAMPLES),
                int(glow::MAX_INTEGER_SAMPLES),
                int(glow::MAX_COLOR_TEXTURE_SAMPLES),
                if storage {
                    int(glow::MAX_IMAGE_SAMPLES)
                } else {
                    0
                },
            ];
            let push = gl.create_buffer()?;
            let old = buffer(gl.get_parameter_i32(glow::UNIFORM_BUFFER_BINDING));
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(push));
            gl.buffer_data_size(glow::UNIFORM_BUFFER, 256, glow::DYNAMIC_DRAW);
            gl.bind_buffer(glow::UNIFORM_BUFFER, old);
            let fbo = gl.create_framebuffer()?;
            let read = gl.create_framebuffer()?;
            let resolve = Resolve::new(&gl)?;
            Ok(Self {
                gl,
                extra,
                version,
                extensions,
                caps,
                anisotropy,
                samples,
                res: Resources::default(),
                push,
                fbo,
                read,
                resolve,
                cache: None,
                units: 0,
            })
        }
    }

    fn has(&self, version: u32, extension: &str) -> bool {
        has(self.version, &self.extensions, version, extension)
    }

    pub fn caps(&self) -> Caps {
        self.caps
    }

    pub fn name(&self) -> String {
        unsafe { self.gl.get_parameter_string(glow::RENDERER) }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.caps.buffer(size, memory)?;
        let persistent = self.extra.storage.is_some();
        let gl = &self.gl;
        let (flags, usage) = match memory {
            memory::DEVICE => (0, glow::STATIC_DRAW),
            memory::UPLOAD => (glow::MAP_WRITE_BIT, glow::STREAM_DRAW),
            memory::READBACK => (glow::MAP_READ_BIT, glow::STREAM_READ),
            m => panic!("unknown memory {m}"),
        };
        let length =
            isize::try_from(size).map_err(|_| format!("buffer size {size} is too large"))?;
        let target = glow::COPY_WRITE_BUFFER;
        unsafe {
            drain(gl);
            let raw = gl.create_buffer()?;
            let old = buffer(gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            gl.bind_buffer(target, Some(raw));
            let mut data: *mut u8 = std::ptr::null_mut();
            if let Some(storage) = self.extra.storage {
                let access = if flags == 0 {
                    0
                } else {
                    flags | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT
                };
                let bits = access | glow::DYNAMIC_STORAGE_BIT;
                storage(target, length, std::ptr::null(), bits);
                if flags != 0 {
                    data = (self.extra.map)(target, 0, length, access).cast();
                }
            } else {
                (self.extra.data)(target, length, std::ptr::null(), usage);
            }
            gl.bind_buffer(target, old);
            let error = gl.get_error();
            if error != glow::NO_ERROR || (flags != 0 && persistent && data.is_null()) {
                gl.delete_buffer(raw);
                return Err(format!(
                    "buffer allocation of {size} bytes failed ({error:#x})"
                ));
            }
            let id = self.res.buffers.insert(Buffer {
                raw,
                size,
                memory,
                data: Cell::new(data),
                persistent,
                own: true,
            });
            self.res.table.buffers.set(id, size);
            Ok(id)
        }
    }

    pub fn import_buffer(&mut self, raw: u32, size: u64) -> u32 {
        let raw = glow::NativeBuffer(std::num::NonZeroU32::new(raw).expect("buffer 0"));
        let id = self.res.buffers.insert(Buffer {
            raw,
            size,
            memory: memory::DEVICE,
            data: Cell::new(std::ptr::null_mut()),
            persistent: false,
            own: false,
        });
        self.res.table.buffers.set(id, size);
        id
    }

    pub fn raw_buffer(&self, buffer: u32) -> u32 {
        self.res.buffers.get(buffer).raw.0.get()
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.res.table.buffers.clear(buffer);
        let b = self.res.buffers.remove(buffer);
        if b.own {
            unsafe { self.gl.delete_buffer(b.raw) };
        }
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        let b = self.res.buffers.get(buffer);
        assert!(b.memory != memory::DEVICE, "map of a device buffer");
        if b.persistent || !b.data.get().is_null() {
            return b.data.get();
        }
        let access = match b.memory {
            memory::UPLOAD => glow::MAP_WRITE_BIT | glow::MAP_UNSYNCHRONIZED_BIT,
            _ => glow::MAP_READ_BIT,
        };
        unsafe {
            let old = state::buffer(self.gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(b.raw));
            let data = (self.extra.map)(glow::COPY_WRITE_BUFFER, 0, b.size as isize, access);
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, old);
            b.data.set(data.cast());
        }
        b.data.get()
    }

    pub fn unmap(&self, buffer: u32) {
        let b = self.res.buffers.get(buffer);
        if b.persistent || b.data.get().is_null() {
            return;
        }
        unsafe {
            let old = state::buffer(self.gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(b.raw));
            self.gl.unmap_buffer(glow::COPY_WRITE_BUFFER);
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, old);
        }
        b.data.set(std::ptr::null_mut());
    }

    pub fn buffer_handle(&self, _buffer: u32) -> Result<u64, String> {
        Err("GL has no bindless heap".into())
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        if !self.has(43, "GL_ARB_compute_shader") {
            return Err("compute shaders need GL 4.3".into());
        }
        let program = Program::new(
            &self.gl,
            &self.extra,
            info.code,
            &[(glow::COMPUTE_SHADER, info.entry)],
            self.cache.as_mut(),
        )?;
        self.units = self.units.max(program.pairs.len() as u32);
        let id = self.res.pipelines.insert(program);
        self.res.table.pipelines.set(id, ());
        Ok(id)
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        self.res.table.pipelines.clear(pipeline);
        let p = self.res.pipelines.remove(pipeline);
        unsafe { self.gl.delete_program(p.raw) };
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        if !uniform(info.colors) && !self.caps.blends {
            return Err("different blend states per color target need GL 4.0".into());
        }
        if info.views > 1 {
            return Err("multiview is not supported on GL".into());
        }
        let program = Program::new(
            &self.gl,
            &self.extra,
            info.code,
            &[
                (glow::VERTEX_SHADER, info.vertex),
                (glow::FRAGMENT_SHADER, info.fragment),
            ],
            self.cache.as_mut(),
        )?;
        self.units = self.units.max(program.pairs.len() as u32);
        let vao = match unsafe { self.gl.create_vertex_array() } {
            Ok(v) => v,
            Err(e) => {
                unsafe { self.gl.delete_program(program.raw) };
                return Err(e);
            }
        };
        let id = self.res.renders.insert(Pipeline {
            program,
            vao,
            buffers: info.buffers.to_vec(),
            attributes: info.attributes.to_vec(),
            colors: info.colors.to_vec(),
            topology: info.topology,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
        });
        self.res.table.renders.set(id, ());
        Ok(id)
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        if !uniform(info.colors) && !self.caps.blends {
            return Err("different blend states per color target need GL 4.0".into());
        }
        if !self.caps.mesh {
            return Err("GL_EXT_mesh_shader is not supported".into());
        }
        if info.views > 1 {
            return Err("multiview is not supported on GL".into());
        }
        let mut stages = Vec::new();
        if !info.task.is_empty() {
            stages.push((TASK_SHADER, info.task));
        }
        stages.push((MESH_SHADER, info.mesh));
        stages.push((glow::FRAGMENT_SHADER, info.fragment));
        let program = Program::new(
            &self.gl,
            &self.extra,
            info.code,
            &stages,
            self.cache.as_mut(),
        )?;
        self.units = self.units.max(program.pairs.len() as u32);
        let vao = match unsafe { self.gl.create_vertex_array() } {
            Ok(v) => v,
            Err(e) => {
                unsafe { self.gl.delete_program(program.raw) };
                return Err(e);
            }
        };
        let id = self.res.renders.insert(Pipeline {
            program,
            vao,
            buffers: Vec::new(),
            attributes: Vec::new(),
            colors: info.colors.to_vec(),
            topology: crate::topology::TRIANGLE,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
        });
        self.res.table.renders.set(id, ());
        Ok(id)
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.res.table.renders.clear(pipeline);
        let p = self.res.renders.remove(pipeline);
        unsafe {
            self.gl.delete_program(p.program.raw);
            self.gl.delete_vertex_array(p.vao);
        }
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)?;
        let [internal, layout, kind_of] = format::gl(info.format);
        let kind = target(info.dimension, info.samples);
        let gl = &self.gl;
        let (w, h, d) = (info.width as i32, info.height as i32, info.depth as i32);
        let (mips, layers) = (info.mips as i32, info.layers as i32);
        let samples = info.samples as i32;
        let storage = self.has(42, "GL_ARB_texture_storage");
        unsafe {
            drain(gl);
            let raw = gl.create_texture()?;
            let old = gl.get_parameter_i32(binding(kind)) as u32;
            let unpack = buffer(gl.get_parameter_i32(glow::PIXEL_UNPACK_BUFFER_BINDING));
            gl.bind_texture(kind, Some(raw));
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, None);
            let multisample = matches!(
                kind,
                glow::TEXTURE_2D_MULTISAMPLE | glow::TEXTURE_2D_MULTISAMPLE_ARRAY
            );
            if !storage && !multisample {
                let [bytes, bw, bh] = format::block(info.format);
                let internal = internal as i32;
                for m in 0..mips {
                    let (w, h, d) = ((w >> m).max(1), (h >> m).max(1), (d >> m).max(1));
                    let size = (w as u32).div_ceil(bw) * (h as u32).div_ceil(bh) * bytes;
                    let size = size as i32;
                    let data = vec![0; (size * d.max(layers)) as usize];
                    let none = glow::PixelUnpackData::Slice(None);
                    match kind {
                        glow::TEXTURE_1D if layout == 0 => {
                            gl.compressed_tex_image_1d(kind, m, internal, w, 0, size, &data)
                        }
                        glow::TEXTURE_1D => {
                            gl.tex_image_1d(kind, m, internal, w, 0, layout, kind_of, none)
                        }
                        glow::TEXTURE_2D | glow::TEXTURE_1D_ARRAY | glow::TEXTURE_CUBE_MAP => {
                            let h = if kind == glow::TEXTURE_1D_ARRAY {
                                layers
                            } else {
                                h
                            };
                            let faces = if kind == glow::TEXTURE_CUBE_MAP { 6 } else { 1 };
                            for f in 0..faces {
                                let face = if faces == 6 {
                                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + f
                                } else {
                                    kind
                                };
                                if layout == 0 {
                                    gl.compressed_tex_image_2d(
                                        face, m, internal, w, h, 0, size, &data,
                                    );
                                } else {
                                    let none = glow::PixelUnpackData::Slice(None);
                                    gl.tex_image_2d(
                                        face, m, internal, w, h, 0, layout, kind_of, none,
                                    );
                                }
                            }
                        }
                        _ => {
                            let d = if kind == glow::TEXTURE_3D { d } else { layers };
                            if layout == 0 {
                                gl.compressed_tex_image_3d(
                                    kind,
                                    m,
                                    internal,
                                    w,
                                    h,
                                    d,
                                    0,
                                    size * d,
                                    &data,
                                );
                            } else {
                                gl.tex_image_3d(
                                    kind, m, internal, w, h, d, 0, layout, kind_of, none,
                                );
                            }
                        }
                    }
                }
                gl.tex_parameter_i32(kind, glow::TEXTURE_MAX_LEVEL, mips - 1);
            } else {
                match kind {
                    glow::TEXTURE_1D => gl.tex_storage_1d(kind, mips, internal, w),
                    glow::TEXTURE_1D_ARRAY => gl.tex_storage_2d(kind, mips, internal, w, layers),
                    glow::TEXTURE_2D | glow::TEXTURE_CUBE_MAP => {
                        gl.tex_storage_2d(kind, mips, internal, w, h)
                    }
                    glow::TEXTURE_2D_MULTISAMPLE => {
                        let f = self.extra.storage2.unwrap_or(self.extra.samples2);
                        f(kind, samples, internal, w, h, 1)
                    }
                    glow::TEXTURE_2D_MULTISAMPLE_ARRAY => {
                        let f = self.extra.storage3.unwrap_or(self.extra.samples3);
                        f(kind, samples, internal, w, h, layers, 1)
                    }
                    glow::TEXTURE_3D => gl.tex_storage_3d(kind, mips, internal, w, h, d),
                    _ => gl.tex_storage_3d(kind, mips, internal, w, h, layers),
                }
            }
            gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, unpack);
            gl.bind_texture(
                kind,
                std::num::NonZeroU32::new(old).map(glow::NativeTexture),
            );
            if gl.get_error() != glow::NO_ERROR {
                gl.delete_texture(raw);
                return Err("texture allocation failed".into());
            }
            let id = self.res.textures.insert(Texture {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
            });
            self.res.table.textures.set(id, *info);
            Ok(id)
        }
    }

    pub fn import_texture(&mut self, raw: u32, info: &TextureInfo) -> Result<u32, String> {
        let id = self.res.textures.insert(Texture {
            raw: std::num::NonZeroU32::new(raw).map(glow::NativeTexture),
            target: target(info.dimension, info.samples),
            format: info.format,
            own: false,
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn raw_texture(&self, texture: u32) -> u32 {
        self.res.textures.get(texture).raw.map_or(0, |t| t.0.get())
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        let s3tc = self.has(99, "GL_EXT_texture_compression_s3tc");
        let present = match format {
            format::BC1_UNORM | format::BC2_UNORM | format::BC3_UNORM => s3tc,
            format::BC1_SRGB | format::BC2_SRGB | format::BC3_SRGB => {
                s3tc && self.has(99, "GL_EXT_texture_sRGB")
            }
            format::BC6H_UFLOAT..=format::BC7_SRGB => {
                self.has(42, "GL_ARB_texture_compression_bptc")
            }
            format::ETC2_RGB8_UNORM..=format::EAC_RG11_SNORM => {
                self.has(43, "GL_ARB_ES3_compatibility")
            }
            format::ASTC_4X4_UNORM..=format::ASTC_12X12_FLOAT => {
                self.has(99, "GL_KHR_texture_compression_astc_ldr")
            }
            format::B5G6R5_UNORM => self.has(41, "GL_ARB_ES2_compatibility"),
            format::S8_UINT => self.has(44, "GL_ARB_texture_stencil8"),
            _ => true,
        };
        let [internal, kind, _] = format::gl(format);
        if !present || internal == 0 {
            return 0;
        }
        let views = self.has(43, "GL_ARB_texture_view") && self.has(42, "GL_ARB_texture_storage");
        let base = usage::SAMPLED | if views { usage::VIEW } else { 0 };
        let copies = usage::COPY_SRC | usage::COPY_DST;
        match kind {
            0 => base | usage::COPY_DST,
            glow::DEPTH_STENCIL if format == format::D24_UNORM_S8 => base | usage::DEPTH,
            glow::DEPTH_COMPONENT | glow::DEPTH_STENCIL | glow::STENCIL_INDEX => {
                base | usage::DEPTH | copies
            }
            _ => {
                let unrenderable = matches!(
                    format,
                    format::R8_SNORM
                        | format::RG8_SNORM
                        | format::RGBA8_SNORM
                        | format::R16_SNORM
                        | format::RG16_SNORM
                        | format::RGBA16_SNORM
                        | format::RGB9E5_FLOAT
                        | format::RGB32_UINT
                        | format::RGB32_SINT
                        | format::RGB32_FLOAT
                );
                let unstorable = matches!(
                    format,
                    format::RGBA8_SRGB
                        | format::BGRA8_SRGB
                        | format::B5G6R5_UNORM
                        | format::RGB9E5_FLOAT
                        | format::RGB32_UINT
                        | format::RGB32_SINT
                        | format::RGB32_FLOAT
                );
                let mut u = base | usage::COPY_DST;
                if !unrenderable {
                    u |= usage::COLOR | usage::COPY_SRC;
                }
                if !unstorable && self.has(43, "GL_ARB_shader_storage_buffer_object") {
                    u |= usage::STORAGE;
                }
                u
            }
        }
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        if self.format_usage(format) & usage != usage {
            return 0;
        }
        let [depth, integer, color, image] = self.samples;
        let mut max = match format::gl(format)[1] {
            0 => 1,
            glow::DEPTH_COMPONENT | glow::DEPTH_STENCIL | glow::STENCIL_INDEX => depth,
            _ if format::uint(format) || format::sint(format) => integer,
            _ => color,
        };
        if usage & usage::STORAGE != 0 {
            max = max.min(image);
        }
        (0..7)
            .map(|i| 1 << i)
            .filter(|&n| n <= max.max(1))
            .fold(0, |m, n| m | n as u32)
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let view = self.extra.view.ok_or("texture views need GL 4.3")?;
        let parent = self.res.textures.get(texture);
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        caps::supports(info.format, info.usage | usage::VIEW, 1, usable, counts)?;
        let [internal, _, _] = format::gl(info.format);
        let multisample = matches!(
            parent.target,
            glow::TEXTURE_2D_MULTISAMPLE | glow::TEXTURE_2D_MULTISAMPLE_ARRAY
        );
        let kind = target(info.dimension, if multisample { 2 } else { 1 });
        let gl = &self.gl;
        unsafe {
            drain(gl);
            let raw = gl.create_texture()?;
            view(
                raw.0.get(),
                kind,
                parent.raw.map_or(0, |t| t.0.get()),
                internal,
                info.base_mip,
                info.mips,
                info.base_layer,
                info.layers,
            );
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                gl.delete_texture(raw);
                return Err(format!("texture view failed ({error:#x})"));
            }
            let id = self.res.textures.insert(Texture {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
            });
            self.res.table.view(id, texture, info);
            Ok(id)
        }
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.res.table.textures.clear(texture);
        let t = self.res.textures.remove(texture);
        if t.own
            && let Some(raw) = t.raw
        {
            unsafe { self.gl.delete_texture(raw) };
        }
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        let gl = &self.gl;
        let anisotropy = if info.anisotropy > 1 {
            (info.anisotropy as f32).min(self.anisotropy)
        } else {
            1.0
        };
        let filter = |f| match f {
            filter::NEAREST => glow::NEAREST,
            filter::LINEAR => glow::LINEAR,
            f => panic!("unknown filter {f}"),
        };
        let min = match (info.min, info.mip) {
            (filter::NEAREST, filter::NEAREST) => glow::NEAREST_MIPMAP_NEAREST,
            (filter::NEAREST, filter::LINEAR) => glow::NEAREST_MIPMAP_LINEAR,
            (filter::LINEAR, filter::NEAREST) => glow::LINEAR_MIPMAP_NEAREST,
            (filter::LINEAR, filter::LINEAR) => glow::LINEAR_MIPMAP_LINEAR,
            (m, p) => panic!("unknown filter {m} {p}"),
        };
        let address = |a| match a {
            address::REPEAT => glow::REPEAT,
            address::MIRROR => glow::MIRRORED_REPEAT,
            address::CLAMP => glow::CLAMP_TO_EDGE,
            address::BORDER => glow::CLAMP_TO_BORDER,
            address::MIRROR_CLAMP => glow::MIRROR_CLAMP_TO_EDGE,
            a => panic!("unknown address {a}"),
        } as i32;
        let color = match info.border {
            border::TRANSPARENT_BLACK => [0.0, 0.0, 0.0, 0.0],
            border::OPAQUE_BLACK => [0.0, 0.0, 0.0, 1.0],
            border::OPAQUE_WHITE => [1.0, 1.0, 1.0, 1.0],
            b => panic!("unknown border {b}"),
        };
        unsafe {
            let raw = gl.create_sampler()?;
            gl.sampler_parameter_i32(raw, glow::TEXTURE_MIN_FILTER, min as i32);
            gl.sampler_parameter_i32(raw, glow::TEXTURE_MAG_FILTER, filter(info.mag) as i32);
            gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_S, address(info.u));
            gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_T, address(info.v));
            gl.sampler_parameter_i32(raw, glow::TEXTURE_WRAP_R, address(info.w));
            gl.sampler_parameter_f32(raw, glow::TEXTURE_MIN_LOD, info.min_lod);
            gl.sampler_parameter_f32(raw, glow::TEXTURE_MAX_LOD, info.max_lod);
            gl.sampler_parameter_f32_slice(raw, glow::TEXTURE_BORDER_COLOR, &color);
            if anisotropy > 1.0 {
                gl.sampler_parameter_f32(raw, MAX_ANISOTROPY, anisotropy);
            }
            if info.compare != crate::compare::NONE {
                let mode = glow::COMPARE_REF_TO_TEXTURE as i32;
                gl.sampler_parameter_i32(raw, glow::TEXTURE_COMPARE_MODE, mode);
                gl.sampler_parameter_i32(
                    raw,
                    glow::TEXTURE_COMPARE_FUNC,
                    compare(info.compare) as i32,
                );
            }
            let id = self.res.samplers.insert(raw);
            self.res.table.samplers.set(id, ());
            Ok(id)
        }
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.res.table.samplers.clear(sampler);
        let s = self.res.samplers.remove(sampler);
        unsafe { self.gl.delete_sampler(s) };
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        if !self.caps.cache {
            return Err("program binaries are not supported".into());
        }
        self.cache.get_or_insert_default();
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        let cache = self.cache.as_ref().ok_or("record_cache was not called")?;
        std::fs::write(path, cache.encode()).map_err(|e| e.to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        if !self.caps.cache {
            return Err("program binaries are not supported".into());
        }
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        self.cache = Some(Cache::decode(&data)?);
        Ok(())
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let results = self.extra.results.is_some();
        match kind {
            query::TIMESTAMP | query::OCCLUSION | query::PRECISE if !results => {
                return Err("query results need GL 4.4".into());
            }
            query::TIMESTAMP | query::OCCLUSION | query::PRECISE => {}
            query::STATISTICS => return Err("pipeline statistics are not supported".into()),
            query::COMPACTED => {
                return Err("acceleration structures are not supported on GL".into());
            }
            k => panic!("unknown query kind {k}"),
        }
        let ids = (0..count)
            .map(|_| unsafe { self.gl.create_query() })
            .collect::<Result<_, _>>()?;
        let id = self.res.queries.insert(Query { kind, ids });
        self.res.table.heaps.set(id, (kind, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        let q = self.res.queries.remove(heap);
        for id in q.ids {
            unsafe { self.gl.delete_query(id) };
        }
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        Ok(self.res.events.insert(Event {
            fences: Vec::new(),
            value: 0,
        }))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let e = self.res.events.remove(event);
        for (_, f) in e.fences {
            unsafe { self.gl.delete_sync(f) };
        }
    }

    fn healthy(&self) -> Result<(), String> {
        match self.extra.reset.map(|f| unsafe { f() }) {
            Some(0) | None => Ok(()),
            Some(status) => Err(format!("GL context lost ({status:#x})")),
        }
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.healthy()?;
        let e = self.res.events.get(event);
        let mut value = e.value;
        for &(v, f) in &e.fences {
            if unsafe { self.gl.get_sync_status(f) } != glow::SIGNALED {
                break;
            }
            value = v;
        }
        Ok(value)
    }

    pub fn wait(&mut self, event: u32, value: u64, ms: u64) -> Result<bool, String> {
        self.healthy()?;
        let e = self.res.events.get(event);
        if e.value >= value {
            return Ok(true);
        }
        let Some(&(_, fence)) = e.fences.iter().find(|(v, _)| *v >= value) else {
            return Ok(false);
        };
        let mut left = ms.saturating_mul(1_000_000);
        loop {
            let step = left.min(1_000_000_000);
            let status = unsafe {
                self.gl
                    .client_wait_sync(fence, glow::SYNC_FLUSH_COMMANDS_BIT, step as i32)
            };
            match status {
                glow::ALREADY_SIGNALED | glow::CONDITION_SATISFIED => return Ok(true),
                glow::WAIT_FAILED => return Err("glClientWaitSync failed".into()),
                _ if left <= step => return Ok(false),
                _ => left -= step,
            }
        }
    }

    pub(crate) fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        assert!(queue == 0, "GL has one queue");
        Check::run(commands, natives.len(), &self.caps, &self.res.table)?;
        if let Some(i) = self
            .res
            .buffers
            .find(|b| !b.persistent && !b.data.get().is_null())
        {
            panic!("submit while buffer {i} is mapped; unmap it first");
        }
        self.healthy()?;
        for &(event, value) in waits {
            let e = self.res.events.get(event);
            if e.value >= value {
                continue;
            }
            let &(_, fence) = e
                .fences
                .iter()
                .find(|(v, _)| *v >= value)
                .expect("wait on an event value no submit signals");
            unsafe { self.gl.wait_sync(fence, 0, glow::TIMEOUT_IGNORED) };
        }
        let state = State::save(
            &self.gl,
            &self.extra,
            &self.caps,
            &|v, e| self.has(v, e),
            self.units.max(1),
        );
        state.clear(&self.gl);
        let snapshot = || {
            State::save(
                &self.gl,
                &self.extra,
                &self.caps,
                &|v, e| self.has(v, e),
                self.units.max(1),
            )
        };
        let mut recorder = Recorder::new(
            &self.gl,
            &self.extra,
            &self.res,
            &self.caps,
            (self.push, self.fbo, self.read),
            (&self.resolve, &snapshot),
            natives,
        );
        let result = catch_unwind(AssertUnwindSafe(|| recorder.run(commands)));
        recorder.finish();
        state.restore(&self.gl, &self.extra);
        result.unwrap_or_else(|p| resume_unwind(p))?;
        for &(event, value) in signals {
            let fence = unsafe { self.gl.fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0)? };
            let e = self.res.events.get_mut(event);
            let gl = &self.gl;
            let done: Vec<usize> = e
                .fences
                .iter()
                .enumerate()
                .filter(|(_, (_, f))| unsafe { gl.get_sync_status(*f) } == glow::SIGNALED)
                .map(|(i, _)| i)
                .collect();
            for i in done.into_iter().rev() {
                let (v, f) = e.fences.remove(i);
                e.value = e.value.max(v);
                unsafe { gl.delete_sync(f) };
            }
            e.fences.push((value, fence));
        }
        unsafe { self.gl.flush() };
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.finish();
            for b in self.res.buffers.drain() {
                if b.own {
                    gl.delete_buffer(b.raw);
                }
            }
            for t in self.res.textures.drain() {
                if let (true, Some(raw)) = (t.own, t.raw) {
                    gl.delete_texture(raw);
                }
            }
            for s in self.res.samplers.drain() {
                gl.delete_sampler(s);
            }
            for p in self.res.pipelines.drain() {
                gl.delete_program(p.raw);
            }
            for p in self.res.renders.drain() {
                gl.delete_program(p.program.raw);
                gl.delete_vertex_array(p.vao);
            }
            for q in self.res.queries.drain() {
                for id in q.ids {
                    gl.delete_query(id);
                }
            }
            for e in self.res.events.drain() {
                for (_, f) in e.fences {
                    gl.delete_sync(f);
                }
            }
            gl.delete_buffer(self.push);
            gl.delete_framebuffer(self.fbo);
            gl.delete_framebuffer(self.read);
            self.resolve.delete(gl);
        }
    }
}
