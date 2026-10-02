use crate::gl::buffer::Buffer;
use crate::gl::cache::Cache;
use crate::gl::event::Event;
use crate::gl::extra::Extra;
use crate::gl::native::Callback;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::recorder::Recorder;
use crate::gl::resources::Resources;
use crate::gl::texture::Texture;
use crate::{
    Caps, Commands, ComputeInfo, DepthInfo, MeshInfo, RenderInfo, SamplerInfo, TextureInfo,
    ViewInfo, address, border, dimension, filter, format, memory, query,
};
use glow::HasContext;
use std::cell::Cell;
use std::ffi::{c_char, c_void};

pub struct Device {
    pub gl: glow::Context,
    extra: Extra,
    version: u32,
    extensions: Vec<String>,
    res: Resources,
    push: glow::Buffer,
    fbo: glow::Framebuffer,
    read: glow::Framebuffer,
    mapped: Cell<u32>,
    cache: Option<Cache>,
}

const MAX_ANISOTROPY: u32 = 0x84FE;

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
            let extra = Extra::load(proc);
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
            let push = gl.create_buffer()?;
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(push));
            gl.buffer_data_size(glow::UNIFORM_BUFFER, 256, glow::DYNAMIC_DRAW);
            gl.bind_buffer(glow::UNIFORM_BUFFER, None);
            let fbo = gl.create_framebuffer()?;
            let read = gl.create_framebuffer()?;
            Ok(Self {
                gl,
                extra,
                version,
                extensions,
                res: Resources::default(),
                push,
                fbo,
                read,
                mapped: Cell::new(0),
                cache: None,
            })
        }
    }

    fn at(&self, version: u32, extension: &str) -> bool {
        self.version >= version
            || self
                .extensions
                .binary_search_by(|e| e.as_str().cmp(extension))
                .is_ok()
    }

    pub fn caps(&self) -> Caps {
        let gl = &self.gl;
        let alignment = |p| unsafe { gl.get_parameter_i32(p) as u32 };
        let storage = self.at(43, "GL_ARB_shader_storage_buffer_object");
        let results = self.extra.results.is_some();
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
            multi_draw: self.extra.multi.is_some(),
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
            texture_views: self.extra.view.is_some(),
            occlusion: results,
            cache: self.binaries(),
            mesh: self.at(99, "GL_EXT_mesh_shader") && self.extra.mesh.is_some(),
            ..Default::default()
        }
    }

    pub fn name(&self) -> String {
        unsafe { self.gl.get_parameter_string(glow::RENDERER) }
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let persistent = self.at(44, "GL_ARB_buffer_storage");
        let gl = &self.gl;
        let (flags, usage) = match memory {
            memory::DEVICE => (0, glow::STATIC_DRAW),
            memory::UPLOAD => (glow::MAP_WRITE_BIT, glow::STREAM_DRAW),
            memory::READBACK => (glow::MAP_READ_BIT, glow::STREAM_READ),
            memory::TRANSIENT => return Err("memoryless storage is not supported".into()),
            m => panic!("unknown memory {m}"),
        };
        unsafe {
            let raw = gl.create_buffer()?;
            gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(raw));
            let mut data = std::ptr::null_mut();
            if persistent {
                let access = if flags == 0 {
                    0
                } else {
                    flags | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT
                };
                let storage = access | glow::DYNAMIC_STORAGE_BIT;
                gl.buffer_storage(glow::COPY_WRITE_BUFFER, size as i32, None, storage);
                if flags != 0 {
                    data = gl.map_buffer_range(glow::COPY_WRITE_BUFFER, 0, size as i32, access);
                }
            } else {
                gl.buffer_data_size(glow::COPY_WRITE_BUFFER, size as i32, usage);
            }
            gl.bind_buffer(glow::COPY_WRITE_BUFFER, None);
            if gl.get_error() == glow::OUT_OF_MEMORY {
                gl.delete_buffer(raw);
                return Err("buffer allocation failed".into());
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
        if b.persistent || b.memory == memory::DEVICE || !b.data.get().is_null() {
            return b.data.get();
        }
        let access = match b.memory {
            memory::UPLOAD => glow::MAP_WRITE_BIT | glow::MAP_UNSYNCHRONIZED_BIT,
            _ => glow::MAP_READ_BIT,
        };
        unsafe {
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(b.raw));
            let data = self
                .gl
                .map_buffer_range(glow::COPY_WRITE_BUFFER, 0, b.size as i32, access);
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, None);
            b.data.set(data);
        }
        self.mapped.set(self.mapped.get() + 1);
        b.data.get()
    }

    pub fn unmap(&self, buffer: u32) {
        let b = self.res.buffers.get(buffer);
        if b.persistent || b.data.get().is_null() {
            return;
        }
        unsafe {
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(b.raw));
            self.gl.unmap_buffer(glow::COPY_WRITE_BUFFER);
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, None);
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
        Ok(self.res.pipelines.insert(program))
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let p = self.res.pipelines.remove(pipeline);
        unsafe { self.gl.delete_program(p.raw) };
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
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
            return Err("memoryless storage is not supported".into());
        }
        if !self.at(42, "GL_ARB_texture_storage") {
            return Err("textures need ARB_texture_storage".into());
        }
        let [internal, _, _] = format::gl(info.format);
        if internal == 0 {
            return Err(format!("format {} is not supported on GL", info.format));
        }
        let kind = target(info.dimension, info.samples);
        let gl = &self.gl;
        let (w, h, d) = (info.width as i32, info.height as i32, info.depth as i32);
        let (mips, layers) = (info.mips as i32, info.layers as i32);
        unsafe {
            let raw = gl.create_texture()?;
            gl.bind_texture(kind, Some(raw));
            match kind {
                glow::TEXTURE_1D => gl.tex_storage_1d(kind, mips, internal, w),
                glow::TEXTURE_1D_ARRAY => gl.tex_storage_2d(kind, mips, internal, w, layers),
                glow::TEXTURE_2D | glow::TEXTURE_CUBE_MAP => {
                    gl.tex_storage_2d(kind, mips, internal, w, h)
                }
                glow::TEXTURE_2D_MULTISAMPLE => {
                    gl.tex_storage_2d_multisample(kind, info.samples as i32, internal, w, h, true)
                }
                glow::TEXTURE_3D => gl.tex_storage_3d(kind, mips, internal, w, h, d),
                _ => gl.tex_storage_3d(kind, mips, internal, w, h, layers),
            }
            gl.bind_texture(kind, None);
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

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let view = self.extra.view.ok_or("texture views need GL 4.3")?;
        let parent = self.res.textures.get(texture);
        let [internal, _, _] = format::gl(info.format);
        let kind = target(
            info.dimension,
            if parent.target == glow::TEXTURE_2D_MULTISAMPLE {
                2
            } else {
                1
            },
        );
        unsafe {
            let name = self.gl.create_texture()?.0.get();
            view(
                name,
                kind,
                parent.raw.map_or(0, |t| t.0.get()),
                internal,
                info.base_mip,
                info.mips,
                info.base_layer,
                info.layers,
            );
            Ok(self.res.textures.insert(Texture {
                raw: std::num::NonZeroU32::new(name).map(glow::NativeTexture),
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
        if info.anisotropy > 1 && !self.caps().anisotropy {
            return Err("anisotropic filtering is not supported".into());
        }
        let gl = &self.gl;
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
            if info.anisotropy > 1 {
                gl.sampler_parameter_f32(raw, MAX_ANISOTROPY, info.anisotropy as f32);
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

    pub fn event_value(&self, event: u32) -> Result<u64, String> {
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

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> Result<bool, String> {
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
        let mut recorder = Recorder::new(
            &self.gl,
            &self.extra,
            &self.res,
            &caps,
            self.push,
            (self.fbo, self.read),
            natives,
        );
        let result = recorder.run(commands);
        recorder.finish();
        result?;
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
        }
    }
}
