mod budget;
mod copies;
mod host;

use crate::caps;
use crate::check::Check;
use crate::gl::addresses::GPU_ADDRESS;
use crate::gl::buffer::Buffer;
use crate::gl::cache::Cache;
use crate::gl::event::Event;
use crate::gl::extra::Extra;
use crate::gl::native::Callback;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::recorder::Recorder;
use crate::gl::resolve::Resolve;
use crate::gl::resources::Resources;
use crate::gl::sampler;
use crate::gl::state::{self, State, buffer};
use crate::gl::texture::{Texture, target};
use crate::{
    Caps, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, SamplerInfo, TextureInfo, ViewInfo,
    dimension, format, memory, query, topology, usage,
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
    thread: std::thread::ThreadId,
}

pub const PUSH: usize = 256;

const TASK_SHADER: u32 = 0x955A;

const MESH_SHADER: u32 = 0x9559;

pub fn compare(c: u32) -> u32 {
    match c {
        crate::compare::NONE => glow::ALWAYS,
        crate::compare::NEVER..=crate::compare::ALWAYS => glow::NEVER + c - 1,
        c => panic!("unknown compare {c}"),
    }
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
        if proc == 0 {
            return Err("null GL loader".into());
        }
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
        if proc("glGetIntegerv").is_null() {
            return Err("the GL loader returned no glGetIntegerv".into());
        }
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
                push: PUSH as u32,
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
                indirect_count: has(42, "GL_ARB_draw_indirect")
                    && has(42, "GL_ARB_base_instance")
                    && has(46, "GL_ARB_indirect_parameters"),
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
                max_queries: 65536,
                cube_arrays: has(40, "GL_ARB_texture_cube_map_array"),
                mirror_clamp: has(44, "GL_ARB_texture_mirror_clamp_to_edge"),
                copy_formats: has(43, "GL_ARB_copy_image")
                    && has(43, "GL_ARB_texture_view")
                    && has(42, "GL_ARB_texture_storage"),
                texture_clear: has(44, "GL_ARB_clear_texture"),
                max_buffer: isize::MAX as u64,
                max_texture: int(glow::MAX_TEXTURE_SIZE) as u32,
                max_cube: int(glow::MAX_CUBE_MAP_TEXTURE_SIZE) as u32,
                max_volume: int(glow::MAX_3D_TEXTURE_SIZE) as u32,
                max_layers: int(glow::MAX_ARRAY_TEXTURE_LAYERS) as u32,
                mesh: extra.mesh.is_some(),
                budget: has(99, "GL_NVX_gpu_memory_info") || has(99, "GL_ATI_meminfo"),
                host_read: has(45, "GL_ARB_get_texture_sub_image"),
                buffers: if extra.addresses.is_some() {
                    u32::MAX
                } else {
                    0
                },
                addresses: extra.addresses.is_some(),
                shading_rate: extra.rates.is_some(),
                images: if extra.bindless.is_some() {
                    u32::MAX
                } else {
                    0
                },
                ..Default::default()
            };
            let anisotropy = if caps.anisotropy {
                gl.get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY)
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
            gl.buffer_data_size(glow::UNIFORM_BUFFER, PUSH as i32, glow::DYNAMIC_DRAW);
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
                thread: std::thread::current().id(),
            })
        }
    }

    fn thread(&self) {
        assert_eq!(
            std::thread::current().id(),
            self.thread,
            "GL device used from another thread"
        );
    }

    fn has(&self, version: u32, extension: &str) -> bool {
        has(self.version, &self.extensions, version, extension)
    }

    pub fn caps(&self) -> Caps {
        self.thread();
        self.caps
    }

    pub fn name(&self) -> String {
        self.thread();
        unsafe { self.gl.get_parameter_string(glow::RENDERER) }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.thread();
        self.caps.buffer(size, memory)?;
        let id = self
            .res
            .buffers
            .insert(Buffer::new(&self.gl, &self.extra, size, memory)?);
        self.res.table.buffers.set(id, size);
        Ok(id)
    }

    pub fn import_buffer(&mut self, raw: u32, size: u64) -> u32 {
        self.thread();
        let raw = glow::NativeBuffer(std::num::NonZeroU32::new(raw).expect("buffer 0"));
        let id = self.res.buffers.insert(Buffer {
            raw,
            size,
            memory: memory::DEVICE,
            data: Cell::new(std::ptr::null_mut()),
            persistent: false,
            own: false,
            address: Cell::new(0),
        });
        self.res.table.buffers.set(id, size);
        id
    }

    pub fn raw_buffer(&self, buffer: u32) -> u32 {
        self.thread();
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer).raw.0.get()
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        self.thread();
        self.res.table.buffers.clear(buffer);
        self.res.buffers.remove(buffer).delete(&self.gl);
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.thread();
        self.res.table.buffers.get(buffer);
        let b = self.res.buffers.get(buffer);
        assert!(b.own, "map of an imported buffer");
        assert_ne!(b.memory, memory::DEVICE, "map of a device buffer");
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
        self.thread();
        self.res.table.buffers.get(buffer);
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

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        self.thread();
        self.res.table.buffers.get(buffer);
        if self.extra.addresses.is_none() {
            return Err("buffer handles need GL_NV_shader_buffer_load".into());
        }
        Ok(self.resident(buffer))
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.thread();
        self.res.table.buffers.get(buffer);
        self.caps.address();
        self.resident(buffer)
    }

    fn resident(&self, buffer: u32) -> u64 {
        let b = self.res.buffers.get(buffer);
        if b.address.get() == 0 {
            let a = self.extra.addresses.as_ref().unwrap();
            let target = glow::COPY_WRITE_BUFFER;
            unsafe {
                let old = state::buffer(self.gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
                self.gl.bind_buffer(target, Some(b.raw));
                (a.resident)(target, glow::READ_WRITE);
                let mut address = 0;
                (a.parameter)(target, GPU_ADDRESS, &mut address);
                self.gl.bind_buffer(target, old);
                b.address.set(address);
            }
        }
        b.address.get()
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        self.thread();
        if !self.has(43, "GL_ARB_compute_shader") {
            return Err("compute shaders need GL 4.3".into());
        }
        self.programs()?;
        let program = Program::new(
            &self.gl,
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
        self.thread();
        self.res.table.pipelines.clear(pipeline);
        self.res.pipelines.remove(pipeline).delete(&self.gl);
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        self.thread();
        crate::caps::targets(
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        crate::caps::primitive(info.topology);
        let stages = [
            (glow::VERTEX_SHADER, info.vertex),
            (glow::FRAGMENT_SHADER, info.fragment),
        ];
        self.render(info, &stages)
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        self.thread();
        crate::caps::targets(
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
        );
        if !self.caps.mesh {
            return Err("GL_EXT_mesh_shader is not supported".into());
        }
        let mut stages = Vec::new();
        if !info.task.is_empty() {
            stages.push((TASK_SHADER, info.task));
        }
        stages.push((MESH_SHADER, info.mesh));
        stages.push((glow::FRAGMENT_SHADER, info.fragment));
        let render = RenderInfo {
            code: info.code,
            colors: info.colors,
            topology: topology::TRIANGLE,
            fill: info.fill,
            coverage: info.coverage,
            clamp: info.clamp,
            indirect: info.indirect,
            ..Default::default()
        };
        self.render(&render, &stages)
    }

    fn render(&mut self, info: &RenderInfo, stages: &[(u32, &str)]) -> Result<u32, String> {
        self.programs()?;
        let cache = self.cache.as_mut();
        let p = Pipeline::new(&self.gl, cache, self.caps.blends, info, stages)?;
        self.units = self.units.max(p.program.pairs.len() as u32);
        let id = self.res.renders.insert(p);
        self.res
            .table
            .renders
            .set(id, (info.samples, info.indirect));
        Ok(id)
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        self.thread();
        self.res.table.renders.clear(pipeline);
        self.res.renders.remove(pipeline).delete(&self.gl);
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        self.thread();
        self.caps.depth(info);
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.thread();
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        self.thread();
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.texture(info, usable, counts)?;
        let storage = self.has(42, "GL_ARB_texture_storage");
        let texture = Texture::new(&self.gl, &self.extra, info, storage)?;
        let id = self.res.textures.insert(texture);
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn import_texture(&mut self, raw: u32, info: &TextureInfo) -> Result<u32, String> {
        self.thread();
        let id = self.res.textures.insert(Texture {
            raw: std::num::NonZeroU32::new(raw).map(glow::NativeTexture),
            target: target(info.dimension, info.samples),
            format: info.format,
            own: false,
            image: Cell::new(0),
        });
        self.res.table.textures.set(id, *info);
        Ok(id)
    }

    pub fn raw_texture(&self, texture: u32) -> u32 {
        self.thread();
        self.res.table.textures.get(texture);
        self.res.textures.get(texture).raw.map_or(0, |t| t.0.get())
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        self.thread();
        let info = self.res.table.textures.get(texture);
        self.caps.image(texture);
        let t = self.res.textures.get(texture);
        if t.image.get() == 0 {
            let b = self.extra.bindless.as_ref().unwrap();
            let raw = t.raw.map_or(0, |r| r.0.get());
            let layered = info.layers > 1 || info.dimension == dimension::D3;
            let internal = format::gl(t.format)[0];
            unsafe {
                let handle = (b.handle)(raw, 0, layered as u8, 0, internal);
                (b.resident)(handle, glow::READ_WRITE);
                t.image.set(handle);
            }
        }
        t.image.get()
    }

    pub fn format_usage(&self, format: u32) -> u32 {
        self.thread();
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
        let readable = self.extra.compressed_sub.is_some() && self.extra.copy_image.is_some();
        match kind {
            0 if readable => base | copies | usage::HOST,
            0 => base | usage::COPY_DST | usage::HOST,
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
                let mut u = base | usage::COPY_DST | usage::HOST;
                if !unrenderable {
                    u |= usage::COLOR | usage::COPY_SRC | usage::BLIT;
                }
                if !unstorable && self.has(43, "GL_ARB_shader_storage_buffer_object") {
                    u |= usage::STORAGE;
                }
                u
            }
        }
    }

    pub fn format_samples(&self, format: u32, usage: u32) -> u32 {
        self.thread();
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
        self.thread();
        let info = &self.res.table.resolve(texture, info);
        self.caps.view(info.dimension)?;
        let view = self.extra.view.ok_or("texture views need GL 4.3")?;
        let parent = self.res.textures.get(texture);
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        caps::supports(info.format, info.usage | usage::VIEW, 1, usable, counts)?;
        let view = Texture::view(&self.gl, &self.extra, view, parent, info)?;
        let id = self.res.textures.insert(view);
        self.res.table.view(id, texture, info);
        Ok(id)
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        self.thread();
        self.res.table.textures.clear(texture);
        self.res.textures.remove(texture).delete(&self.gl);
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        self.thread();
        self.caps.sampler(info)?;
        let raw = sampler::create(&self.gl, info, self.anisotropy)?;
        let id = self.res.samplers.insert(raw);
        self.res.table.samplers.set(id, ());
        Ok(id)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        self.thread();
        self.res.table.samplers.clear(sampler);
        sampler::delete(&self.gl, self.res.samplers.remove(sampler));
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        self.thread();
        if !self.caps.cache {
            return Err("program binaries are not supported".into());
        }
        self.cache.get_or_insert_default();
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        self.thread();
        let cache = self.cache.as_ref().ok_or("record_cache was not called")?;
        std::fs::write(path, cache.encode()).map_err(|e| e.to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        self.thread();
        if !self.caps.cache {
            return Err("program binaries are not supported".into());
        }
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        self.cache = Some(Cache::decode(&data)?);
        Ok(())
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        self.thread();
        self.caps.queries(kind, count)?;
        let results = self.has(44, "GL_ARB_query_buffer_object");
        match kind {
            query::TIMESTAMP | query::OCCLUSION | query::PRECISE if !results => {
                return Err("query results need GL 4.4".into());
            }
            query::TIMESTAMP | query::OCCLUSION | query::PRECISE => {}
            query::COMPACTED => {
                return Err("acceleration structures are not supported on GL".into());
            }
            k => panic!("unknown query kind {k}"),
        }
        let id = self.res.queries.insert(Query::new(&self.gl, kind, count)?);
        self.res.table.queries.set(id, (kind, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.thread();
        self.res.table.queries.clear(heap);
        self.res.queries.remove(heap).delete(&self.gl);
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        self.thread();
        let id = self.res.events.insert(Event {
            fences: Vec::new(),
            value: 0,
        });
        self.res.table.events.set(id, (0, usize::MAX));
        Ok(id)
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.thread();
        self.res.table.events.clear(event);
        self.res.events.remove(event).delete(&self.gl);
    }

    fn healthy(&self) -> Result<(), String> {
        match self.extra.reset.map(|f| unsafe { f() }) {
            Some(0) | None => Ok(()),
            Some(status) => Err(format!("GL context lost ({status:#x})")),
        }
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.thread();
        self.res.table.events.get(event);
        self.healthy()?;
        Ok(self.res.events.get(event).value(&self.gl))
    }

    pub fn wait(&mut self, event: u32, value: u64, ms: u64) -> Result<bool, String> {
        self.thread();
        self.res.table.events.get(event);
        self.healthy()?;
        self.res.events.get(event).wait(&self.gl, value, ms)
    }

    pub(crate) fn replay(
        &mut self,
        queue: usize,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        self.thread();
        assert_eq!(queue, 0, "GL has one queue");
        Check::run(commands, natives.len(), &self.caps, &self.res.table)?;
        if let Some(i) = self
            .res
            .buffers
            .find(|b| !b.persistent && !b.data.get().is_null())
        {
            return Err(format!("submit while buffer {i} is mapped; unmap it first"));
        }
        self.programs()?;
        self.queries()?;
        self.res.table.submit(queue, waits, signals);
        self.healthy()?;
        self.waits(waits);
        let snapshot = || {
            State::save(
                &self.gl,
                &self.extra,
                &self.caps,
                &|v, e| self.has(v, e),
                self.units.max(1),
            )
        };
        let state = snapshot();
        state.clear(&self.gl, &self.extra);
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
        self.signals(queue, signals)
    }

    fn waits(&self, waits: &[(u32, u64)]) {
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
    }

    fn signals(&mut self, queue: usize, signals: &[(u32, u64)]) -> Result<(), String> {
        for &(event, value) in signals {
            self.res.events.get_mut(event).signal(&self.gl, value)?;
        }
        self.res.table.signal(queue, signals);
        unsafe { self.gl.flush() };
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if std::thread::current().id() != self.thread {
            return;
        }
        let gl = &self.gl;
        unsafe {
            gl.finish();
            for b in self.res.buffers.drain() {
                b.delete(gl);
            }
            for t in self.res.textures.drain() {
                t.delete(gl);
            }
            for s in self.res.samplers.drain() {
                sampler::delete(gl, s);
            }
            for p in self.res.pipelines.drain() {
                p.delete(gl);
            }
            for p in self.res.renders.drain() {
                p.delete(gl);
            }
            for q in self.res.queries.drain() {
                q.delete(gl);
            }
            for e in self.res.events.drain() {
                e.delete(gl);
            }
            gl.delete_buffer(self.push);
            gl.delete_framebuffer(self.fbo);
            gl.delete_framebuffer(self.read);
            self.resolve.delete(gl);
        }
    }
}
