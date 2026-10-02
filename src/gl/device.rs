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
    Caps, Command, Commands, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, SamplerInfo,
    TextureInfo, ViewInfo, address, border, dimension, filter, format, memory, query, usage,
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
    mapped: Cell<u32>,
    cache: Option<Cache>,
    units: u32,
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

fn at(version: u32, extensions: &[String], minimum: u32, extension: &str) -> bool {
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
            let extra = Extra::load(proc, &|v, e| at(version, &extensions, v, e))?;
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
                res: Resources::default(),
                push,
                fbo,
                read,
                resolve,
                mapped: Cell::new(0),
                cache: None,
                units: 0,
            })
        }
    }

    fn at(&self, version: u32, extension: &str) -> bool {
        at(self.version, &self.extensions, version, extension)
    }

    pub fn caps(&self) -> Caps {
        let gl = &self.gl;
        let alignment = |p| unsafe { gl.get_parameter_i32(p) as u32 };
        let storage = self.at(43, "GL_ARB_shader_storage_buffer_object");
        let results = self.at(44, "GL_ARB_query_buffer_object");
        Caps {
            push: 256,
            queues: 1,
            tick: 1.0,
            timestamps: results,
            precise: results,
            viewports: if self.at(99, "GL_ARB_shader_viewport_layer_array") {
                alignment(glow::MAX_VIEWPORTS)
            } else {
                1
            },
            graphics: 1,
            labels: self.at(43, "GL_KHR_debug"),
            anisotropy: self.at(46, "GL_ARB_texture_filter_anisotropic")
                || self.at(99, "GL_EXT_texture_filter_anisotropic"),
            depth_clamp: true,
            line_fill: true,
            multi_draw: self.at(43, "GL_ARB_multi_draw_indirect"),
            slots: true,
            storage,
            persistent: self.at(44, "GL_ARB_buffer_storage"),
            alignment: if storage {
                alignment(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
                    .max(alignment(glow::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT))
            } else {
                alignment(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
            },
            compute: self.at(43, "GL_ARB_compute_shader"),
            texture_views: self.at(43, "GL_ARB_texture_view"),
            occlusion: results,
            cache: self.binaries(),
            bias_clamp: self.at(46, "GL_ARB_polygon_offset_clamp"),
            blends: self.at(40, "GL_ARB_draw_buffers_blend"),
            indirect: self.at(42, "GL_ARB_draw_indirect") && self.at(42, "GL_ARB_base_instance"),
            indirect_ids: false,
            indirect_offset: false,
            mesh: self.at(99, "GL_EXT_mesh_shader") && self.extra.mesh.is_some(),
            ..Default::default()
        }
    }

    pub fn name(&self) -> String {
        unsafe { self.gl.get_parameter_string(glow::RENDERER) }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let persistent = self.extra.storage.is_some();
        let gl = &self.gl;
        let (flags, usage) = match memory {
            memory::DEVICE => (0, glow::STATIC_DRAW),
            memory::UPLOAD => (glow::MAP_WRITE_BIT, glow::STREAM_DRAW),
            memory::READBACK => (glow::MAP_READ_BIT, glow::STREAM_READ),
            memory::TRANSIENT => panic!("transient memory is for textures"),
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
            Ok(self.res.buffers.insert(Buffer {
                raw,
                size,
                memory,
                data: Cell::new(data),
                persistent,
                own: true,
            }))
        }
    }

    pub fn import_buffer(&mut self, raw: u32, size: u64) -> u32 {
        let raw = glow::NativeBuffer(std::num::NonZeroU32::new(raw).expect("buffer 0"));
        self.res.buffers.insert(Buffer {
            raw,
            size,
            memory: memory::DEVICE,
            data: Cell::new(std::ptr::null_mut()),
            persistent: false,
            own: false,
        })
    }

    pub fn raw_buffer(&self, buffer: u32) -> u32 {
        self.res.buffers.get(buffer).raw.0.get()
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let b = self.res.buffers.remove(buffer);
        if !b.persistent && !b.data.get().is_null() {
            self.mapped.set(self.mapped.get() - 1);
        }
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
        if !b.data.get().is_null() {
            self.mapped.set(self.mapped.get() + 1);
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
        self.mapped.set(self.mapped.get() - 1);
    }

    pub fn buffer_handle(&self, _buffer: u32) -> Result<u64, String> {
        Err("GL has no bindless heap".into())
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        if !self.at(43, "GL_ARB_compute_shader") {
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
        Ok(self.res.pipelines.insert(program))
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let p = self.res.pipelines.remove(pipeline);
        unsafe { self.gl.delete_program(p.raw) };
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        if !uniform(info.colors) && !self.caps().blends {
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
        Ok(self.res.renders.insert(Pipeline {
            program,
            vao,
            buffers: info.buffers.to_vec(),
            attributes: info.attributes.to_vec(),
            colors: info.colors.to_vec(),
            topology: info.topology,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
        }))
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        if !uniform(info.colors) && !self.caps().blends {
            return Err("different blend states per color target need GL 4.0".into());
        }
        if !self.caps().mesh {
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
        Ok(self.res.renders.insert(Pipeline {
            program,
            vao,
            buffers: Vec::new(),
            attributes: Vec::new(),
            colors: info.colors.to_vec(),
            topology: crate::topology::TRIANGLE,
            fill: info.fill,
            clamp: info.clamp,
            coverage: info.coverage,
        }))
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        let p = self.res.renders.remove(pipeline);
        unsafe {
            self.gl.delete_program(p.program.raw);
            self.gl.delete_vertex_array(p.vao);
        }
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        Ok(self.res.depths.insert(*info))
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        if info.memory == memory::TRANSIENT {
            assert!(
                info.usage & !(usage::COLOR | usage::DEPTH) == 0,
                "transient textures are attachments only"
            );
            return Err("memoryless storage is not supported".into());
        }
        if !self.at(42, "GL_ARB_texture_storage") {
            return Err("textures need ARB_texture_storage".into());
        }
        let supported = self.format_usage(info.format);
        if supported == 0 {
            return Err(format!("format {} is not supported on GL", info.format));
        }
        if info.usage & !supported != 0 {
            return Err(format!(
                "usage {:#x} is not supported for format {}",
                info.usage & !supported,
                info.format
            ));
        }
        let [internal, _, _] = format::gl(info.format);
        let kind = target(info.dimension, info.samples);
        let gl = &self.gl;
        let (w, h, d) = (info.width as i32, info.height as i32, info.depth as i32);
        let (mips, layers) = (info.mips as i32, info.layers as i32);
        let samples = info.samples as i32;
        unsafe {
            drain(gl);
            let raw = gl.create_texture()?;
            let old = gl.get_parameter_i32(binding(kind)) as u32;
            gl.bind_texture(kind, Some(raw));
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
            gl.bind_texture(
                kind,
                std::num::NonZeroU32::new(old).map(glow::NativeTexture),
            );
            if gl.get_error() != glow::NO_ERROR {
                gl.delete_texture(raw);
                return Err("texture allocation failed".into());
            }
            Ok(self.res.textures.insert(Texture {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
            }))
        }
    }

    pub fn import_texture(&mut self, raw: u32, info: &TextureInfo) -> Result<u32, String> {
        Ok(self.res.textures.insert(Texture {
            raw: std::num::NonZeroU32::new(raw).map(glow::NativeTexture),
            target: target(info.dimension, info.samples),
            format: info.format,
            own: false,
        }))
    }

    pub fn raw_texture(&self, texture: u32) -> u32 {
        self.res.textures.get(texture).raw.map_or(0, |t| t.0.get())
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        let s3tc = self.at(99, "GL_EXT_texture_compression_s3tc");
        let present = match format {
            format::BC1_UNORM | format::BC2_UNORM | format::BC3_UNORM => s3tc,
            format::BC1_SRGB | format::BC2_SRGB | format::BC3_SRGB => {
                s3tc && self.at(99, "GL_EXT_texture_sRGB")
            }
            format::BC6H_UFLOAT..=format::BC7_SRGB => {
                self.at(42, "GL_ARB_texture_compression_bptc")
            }
            format::ETC2_RGB8_UNORM..=format::EAC_RG11_SNORM => {
                self.at(43, "GL_ARB_ES3_compatibility")
            }
            format::ASTC_4X4_UNORM..=format::ASTC_12X12_FLOAT => {
                self.at(99, "GL_KHR_texture_compression_astc_ldr")
            }
            format::B5G6R5_UNORM => self.at(41, "GL_ARB_ES2_compatibility"),
            format::S8_UINT => self.at(44, "GL_ARB_texture_stencil8"),
            _ => true,
        };
        let [internal, kind, _] = format::gl(format);
        if !present || internal == 0 || !self.at(42, "GL_ARB_texture_storage") {
            return 0;
        }
        let base = usage::SAMPLED
            | if self.at(43, "GL_ARB_texture_view") {
                usage::VIEW
            } else {
                0
            };
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
                if !unstorable && self.at(43, "GL_ARB_shader_storage_buffer_object") {
                    u |= usage::STORAGE;
                }
                u
            }
        }
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let view = self.extra.view.ok_or("texture views need GL 4.3")?;
        let parent = self.res.textures.get(texture);
        let supported = self.format_usage(info.format);
        if supported & usage::VIEW == 0 || info.usage & !supported != 0 {
            return Err(format!(
                "usage {:#x} is not supported for a view of format {}",
                info.usage, info.format
            ));
        }
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
            Ok(self.res.textures.insert(Texture {
                raw: Some(raw),
                target: kind,
                format: info.format,
                own: true,
            }))
        }
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        let t = self.res.textures.remove(texture);
        if t.own
            && let Some(raw) = t.raw
        {
            unsafe { self.gl.delete_texture(raw) };
        }
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        let gl = &self.gl;
        let anisotropy = if info.anisotropy > 1 && self.caps().anisotropy {
            let max = unsafe { gl.get_parameter_f32(MAX_ANISOTROPY_LIMIT) };
            (info.anisotropy as f32).min(max)
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
            Ok(self.res.samplers.insert(raw))
        }
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        let s = self.res.samplers.remove(sampler);
        unsafe { self.gl.delete_sampler(s) };
    }

    fn binaries(&self) -> bool {
        unsafe { self.gl.get_parameter_i32(glow::NUM_PROGRAM_BINARY_FORMATS) > 0 }
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        if !self.binaries() {
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
        if !self.binaries() {
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
        Ok(self.res.queries.insert(Query { kind, ids }))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
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

    pub fn wait(&mut self, event: u32, value: u64, timeout: u64) -> Result<bool, String> {
        self.healthy()?;
        let e = self.res.events.get(event);
        if e.value >= value {
            return Ok(true);
        }
        let Some(&(_, fence)) = e.fences.iter().find(|(v, _)| *v >= value) else {
            return Ok(false);
        };
        let mut left = timeout.saturating_mul(1_000_000);
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

    pub fn submit(
        &mut self,
        queue: usize,
        commands: &Commands,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        self.replay(
            queue,
            (commands.bytes(), commands.natives()),
            waits,
            signals,
        )
    }

    pub fn submit_bytes(
        &mut self,
        queue: usize,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        self.replay(queue, (commands, &[]), waits, signals)
    }

    fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        assert!(queue == 0, "GL has one queue");
        Command::check(commands, &self.caps())?;
        assert!(
            self.mapped.get() == 0,
            "submit with {} buffers mapped",
            self.mapped.get()
        );
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
        let caps = self.caps();
        let state = State::save(
            &self.gl,
            &self.extra,
            &caps,
            &|v, e| self.at(v, e),
            self.units.max(1),
        );
        state.clear(&self.gl);
        let snapshot = || {
            State::save(
                &self.gl,
                &self.extra,
                &caps,
                &|v, e| self.at(v, e),
                self.units.max(1),
            )
        };
        let mut recorder = Recorder::new(
            &self.gl,
            &self.extra,
            &self.res,
            &caps,
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
