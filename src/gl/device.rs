mod budget;
mod copies;
mod host;
mod indirect;
mod residency;

use crate::Error;
use crate::caps;
use crate::check::Check;
use crate::gl::buffer::BufferObject;
use crate::gl::extra::Extra;
use crate::gl::fences::Fences;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::recorder::Recorder;
use crate::gl::resources::Resources;
use crate::gl::sampler;
use crate::gl::state::{self, State};
use crate::gl::texture::{TextureObject, target};
use crate::recording::{Held, Recording};
use crate::{
    Atomic64, Caps, ComputePipelineInfo, DepthStateInfo, Format, Memory, MeshPipelineInfo,
    RenderPipelineInfo, SamplerInfo, Stage, SubgroupOp, TextureDimension, TextureInfo,
    TextureUsage, TextureViewInfo, TilePipelineInfo, Timeline, Topology, slot,
};
use crate::{
    Buffer, CommandBuffer, CommandPool, ComputePipeline, DepthState, ImportedPassInfo, QueryPool,
    RenderPass, RenderPipeline, Sampler, Texture, TilePipeline,
};
use glow::HasContext;
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

pub struct Device {
    pub gl: Rc<glow::Context>,
    extra: Extra,
    version: u32,
    extensions: Vec<String>,
    res: Resources,
    fbo: glow::Framebuffer,
    read: glow::Framebuffer,
    open: Cell<bool>,
    units: u32,
    caps: Caps,
    samples: [i32; 4],
    thread: std::thread::ThreadId,
}

const TASK_SHADER: u32 = 0x955A;

const MESH_SHADER: u32 = 0x9559;

const MAX_TASK_TOTAL: u32 = 0x9740;

const MAX_MESH_TOTAL: u32 = 0x9741;

const MAX_TASK_COUNT: u32 = 0x9751;

const MAX_MESH_COUNT: u32 = 0x9752;

const SUBGROUP_SIZE: u32 = 0x9532;

const SUBGROUP_STAGES: u32 = 0x9533;

const SUBGROUP_FEATURES: u32 = 0x9534;

const MESH_SHADER_BIT: u32 = 0x40;

const TASK_SHADER_BIT: u32 = 0x80;

pub fn compare(c: crate::CompareOp) -> u32 {
    match c {
        crate::CompareOp::None => glow::ALWAYS,
        c => glow::NEVER + c as u32 - 1,
    }
}

fn has(version: u32, extensions: &[String], minimum: u32, extension: &str) -> bool {
    version >= minimum
        || extensions
            .binary_search_by(|e| e.as_str().cmp(extension))
            .is_ok()
}

impl Device {
    pub fn import_device(proc: impl Fn(&str) -> *const c_void) -> Result<Self, Error> {
        Self::load(&proc)
    }

    pub fn import_device_raw(proc: u64) -> Result<Self, Error> {
        if proc == 0 {
            return Err(Error::Invalid("null GL loader".into()));
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

    fn load(proc: &dyn Fn(&str) -> *const c_void) -> Result<Self, Error> {
        let proc = &|name: &str| {
            let p = proc(name);
            if matches!(p as isize, 1..=3 | -1) {
                std::ptr::null()
            } else {
                p
            }
        };
        if proc("glGetIntegerv").is_null() {
            return Err(Error::Driver(
                "the GL loader returned no glGetIntegerv".into(),
            ));
        }
        unsafe {
            let gl = glow::Context::from_loader_function(|name| proc(name));
            let major = gl.get_parameter_i32(glow::MAJOR_VERSION) as u32;
            let minor = gl.get_parameter_i32(glow::MINOR_VERSION) as u32;
            let version = major * 10 + minor;
            if version < 33 {
                return Err(Error::Unsupported(format!(
                    "GL {major}.{minor} is below 3.3"
                )));
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
            let compute = has(43, "GL_ARB_compute_shader");
            let groups = has(99, "GL_KHR_shader_subgroup");
            let group = |p| if groups { int(p) as u32 } else { 0 };
            let stages = [
                (glow::VERTEX_SHADER_BIT, Stage::VERTEX),
                (glow::FRAGMENT_SHADER_BIT, Stage::FRAGMENT),
                (glow::COMPUTE_SHADER_BIT, Stage::COMPUTE),
                (MESH_SHADER_BIT, Stage::MESH),
                (TASK_SHADER_BIT, Stage::TASK),
            ]
            .into_iter()
            .filter(|(g, _)| group(SUBGROUP_STAGES) & g != 0)
            .fold(Stage::default(), |m, (_, s)| m | s);
            let caps = Caps {
                queues: 1,
                timestamp_period: 1.0,
                timestamps: results,
                precise_occlusion: results,
                viewports: if has(99, "GL_ARB_shader_viewport_layer_array") {
                    (int(glow::MAX_VIEWPORTS) as u32).min(slot::VIEWPORTS)
                } else {
                    1
                },
                graphics_queues: 1,
                anisotropy: if has(46, "GL_ARB_texture_filter_anisotropic")
                    || has(99, "GL_EXT_texture_filter_anisotropic")
                {
                    gl.get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY) as u32
                } else {
                    1
                },
                depth_clamp: true,
                line_fill: true,
                slot_binding: true,
                storage_slots: storage,
                persistent_maps: has(44, "GL_ARB_buffer_storage"),
                bind_alignment: if storage {
                    int(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT)
                        .max(int(glow::SHADER_STORAGE_BUFFER_OFFSET_ALIGNMENT))
                        as u32
                } else {
                    int(glow::UNIFORM_BUFFER_OFFSET_ALIGNMENT) as u32
                },
                compute,
                texture_views: has(43, "GL_ARB_texture_view"),
                occlusion: results,
                bias_clamp: has(46, "GL_ARB_polygon_offset_clamp"),
                independent_blend: has(40, "GL_ARB_draw_buffers_blend"),
                indirect: has(42, "GL_ARB_draw_indirect") && has(42, "GL_ARB_base_instance"),
                indirect_offset: false,
                max_queries: 65536,
                cube_arrays: has(40, "GL_ARB_texture_cube_map_array"),
                mirror_clamp: has(44, "GL_ARB_texture_mirror_clamp_to_edge"),
                buffer_clear: extra.clear.is_some(),
                partial_resolve: true,
                max_buffer: isize::MAX as u64,
                max_uniform: gl.get_parameter_i64(glow::MAX_UNIFORM_BLOCK_SIZE) as u64,
                max_storage: if storage {
                    gl.get_parameter_i64(glow::MAX_SHADER_STORAGE_BLOCK_SIZE) as u64
                } else {
                    0
                },
                max_texture: int(glow::MAX_TEXTURE_SIZE) as u32,
                max_cube: int(glow::MAX_CUBE_MAP_TEXTURE_SIZE) as u32,
                max_volume: int(glow::MAX_3D_TEXTURE_SIZE) as u32,
                max_layers: int(glow::MAX_ARRAY_TEXTURE_LAYERS) as u32,
                subgroup: group(SUBGROUP_SIZE),
                subgroup_sizes: group(SUBGROUP_SIZE),
                subgroup_ops: SubgroupOp(group(SUBGROUP_FEATURES) & SubgroupOp::BITS),
                subgroup_stages: stages,
                max_threads: if compute {
                    int(glow::MAX_COMPUTE_WORK_GROUP_INVOCATIONS) as u32
                } else {
                    0
                },
                max_shared: if compute {
                    int(glow::MAX_COMPUTE_SHARED_MEMORY_SIZE) as u32
                } else {
                    0
                },
                atomics64: if has(99, "GL_NV_shader_atomic_int64") {
                    Atomic64::MIN_MAX | Atomic64::SIGNED
                } else {
                    Atomic64::default()
                },
                mesh: extra.mesh.is_some(),
                task: extra.mesh.is_some(),
                mesh_indirect: extra.mesh.is_some(),
                indirect_count: false,
                multi_draw: false,
                max_draws: 0,
                mesh_groups: if extra.mesh.is_some() {
                    int(MAX_TASK_TOTAL).min(int(MAX_MESH_TOTAL)) as u32
                } else {
                    0
                },
                mesh_axis: if extra.mesh.is_some() {
                    (0..3)
                        .flat_map(|i| [MAX_TASK_COUNT, MAX_MESH_COUNT].map(|p| (p, i)))
                        .map(|(p, i)| gl.get_parameter_indexed_i32(p, i))
                        .min()
                        .unwrap() as u32
                } else {
                    0
                },
                budget: has(99, "GL_NVX_gpu_memory_info") || has(99, "GL_ATI_meminfo"),
                host_read: has(45, "GL_ARB_get_texture_sub_image"),
                images: if extra.bindless.is_some() {
                    u32::MAX
                } else {
                    0
                },
                ..Default::default()
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
            let fbo = gl.create_framebuffer().map_err(Error::Driver)?;
            let read = gl.create_framebuffer().map_err(Error::Driver)?;
            Ok(Self {
                gl: Rc::new(gl),
                extra,
                version,
                extensions,
                caps,
                samples,
                res: Resources::default(),
                fbo,
                read,
                open: Cell::new(false),
                units: 0,
                thread: std::thread::current().id(),
            })
        }
    }

    fn thread(&self) {
        check_eq!(
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

    pub fn device_name(&self) -> String {
        self.thread();
        unsafe { self.gl.get_parameter_string(glow::RENDERER) }
    }

    pub fn create_buffer(&mut self, size: u64, memory: Memory) -> Result<Buffer, Error> {
        self.thread();
        self.caps.check_buffer(size, memory)?;
        let id = self
            .res
            .buffers
            .insert(BufferObject::new(&self.gl, &self.extra, size, memory)?);
        self.res
            .table
            .add_buffer(id, size, memory != Memory::Device);
        Ok(id)
    }

    pub fn import_buffer(&mut self, raw: u32, size: u64) -> Buffer {
        self.thread();
        let raw = glow::NativeBuffer(std::num::NonZeroU32::new(raw).expect("buffer 0"));
        let id = self.res.buffers.insert(BufferObject {
            raw,
            size,
            memory: Memory::Device,
            data: Cell::new(std::ptr::null_mut()),
            persistent: false,
            own: false,
        });
        self.res.table.add_buffer(id, size, true);
        self.res.table.residency.exempt_buffer(id);
        id
    }

    pub fn raw_buffer(&self, buffer: Buffer) -> u32 {
        self.thread();
        self.res.table.buffers.get(buffer);
        self.res.buffers.get(buffer).raw.0.get()
    }

    pub fn destroy_buffer(&mut self, buffer: Buffer) {
        self.thread();
        self.res.table.drop_buffer(buffer);
        self.res.buffers.remove(buffer).delete(&self.gl);
    }

    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    #[allow(clippy::missing_safety_doc, clippy::mut_from_ref)]
    pub unsafe fn map_buffer<T: Copy>(&self, buffer: Buffer) -> &mut [T] {
        self.thread();
        self.res.table.assert_mappable(buffer);
        let b = self.res.buffers.get(buffer);
        if b.persistent || !b.data.get().is_null() {
            return unsafe { self.res.table.mapping(buffer, b.data.get()) };
        }
        let access = match (b.own, b.memory) {
            (false, _) => glow::MAP_READ_BIT | glow::MAP_WRITE_BIT,
            (_, Memory::Upload) => glow::MAP_WRITE_BIT | glow::MAP_UNSYNCHRONIZED_BIT,
            _ => glow::MAP_READ_BIT,
        };
        unsafe {
            let old = state::buffer(self.gl.get_parameter_i32(glow::COPY_WRITE_BUFFER_BINDING));
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(b.raw));
            let data = (self.extra.map)(glow::COPY_WRITE_BUFFER, 0, b.size as isize, access);
            self.gl.bind_buffer(glow::COPY_WRITE_BUFFER, old);
            b.data.set(data.cast());
        }
        unsafe { self.res.table.mapping(buffer, b.data.get()) }
    }

    pub fn unmap_buffer(&self, buffer: Buffer) {
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

    pub fn buffer_address(&self, buffer: Buffer) -> u64 {
        self.thread();
        self.res.table.buffers.get(buffer);
        self.caps.assert_addresses();
        unreachable!("caps.addresses is false on GL")
    }

    pub fn create_compute_pipeline(
        &mut self,
        info: &ComputePipelineInfo,
    ) -> Result<ComputePipeline, Error> {
        self.thread();
        self.caps.check_compute_pipeline(info)?;
        self.check_host_programs()?;
        let program = Program::new(&self.gl, info.code, &[(glow::COMPUTE_SHADER, info.entry)])?;
        self.units = self.units.max(program.pairs.len() as u32);
        let id = self.res.pipelines.insert(program);
        self.res.table.pipelines.set(id, ());
        Ok(id)
    }

    pub fn destroy_compute_pipeline(&mut self, pipeline: ComputePipeline) {
        self.thread();
        self.res.table.pipelines.clear(pipeline);
        self.res.pipelines.remove(pipeline).delete(&self.gl);
    }

    pub fn create_render_pipeline(
        &mut self,
        info: &RenderPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        self.thread();
        self.caps.check_render_pipeline(info)?;
        let stages = [
            (glow::VERTEX_SHADER, info.vertex),
            (glow::FRAGMENT_SHADER, info.fragment),
        ];
        self.render(info, &stages)
    }

    pub fn create_mesh_pipeline(
        &mut self,
        info: &MeshPipelineInfo,
    ) -> Result<RenderPipeline, Error> {
        self.thread();
        self.caps.check_mesh_pipeline(info)?;
        let mut stages = Vec::new();
        if !info.task.is_empty() {
            stages.push((TASK_SHADER, info.task));
        }
        stages.push((MESH_SHADER, info.mesh));
        stages.push((glow::FRAGMENT_SHADER, info.fragment));
        let render = RenderPipelineInfo {
            code: info.code,
            colors: info.colors,
            topology: Topology::Triangle,
            fill: info.fill,
            coverage: info.coverage,
            clamp: info.clamp,
            ordered: info.ordered,
            indirect: info.indirect,
            ..Default::default()
        };
        self.render(&render, &stages)
    }

    fn render(
        &mut self,
        info: &RenderPipelineInfo,
        stages: &[(u32, &str)],
    ) -> Result<RenderPipeline, Error> {
        self.check_host_programs()?;
        let p = Pipeline::new(&self.gl, info, stages)?;
        self.units = self.units.max(p.program.pairs.len() as u32);
        let id = self.res.renders.insert(p);
        let mesh = stages.iter().any(|s| s.0 == MESH_SHADER);
        self.res.table.renders.set(
            id,
            crate::table::RenderEntry {
                samples: info.samples,
                topology: info.topology,
                mesh,
                indirect: info.indirect,
                vertices: crate::table::vertex_buffer_mask(info),
                reads: 0,
            },
        );
        Ok(id)
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: RenderPipeline) {
        self.thread();
        self.res.table.renders.clear(pipeline);
        self.res.renders.remove(pipeline).delete(&self.gl);
    }

    pub fn create_tile_pipeline(&mut self, info: &TilePipelineInfo) -> Result<TilePipeline, Error> {
        self.thread();
        crate::caps::check_tile_pipeline(info);
        Err(Error::Unsupported(
            "create_tile_pipeline needs caps.tiles".into(),
        ))
    }

    pub fn destroy_tile_pipeline(&mut self, pipeline: TilePipeline) {
        self.thread();
        self.res.table.tile_pipelines.clear(pipeline);
    }

    pub fn create_depth_state(&mut self, info: &DepthStateInfo) -> Result<DepthState, Error> {
        self.thread();
        let id = self.res.depths.insert(*info);
        self.res.table.states.set(id, ());
        Ok(id)
    }

    pub fn destroy_depth_state(&mut self, state: DepthState) {
        self.thread();
        self.res.table.states.clear(state);
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<Texture, Error> {
        self.thread();
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        self.caps.check_texture(info, usable, counts)?;
        let storage = self.has(42, "GL_ARB_texture_storage");
        let texture = TextureObject::new(&self.gl, &self.extra, info, storage)?;
        let id = self.res.textures.insert(texture);
        self.res.table.add_texture(id, info);
        Ok(id)
    }

    pub fn import_texture(&mut self, raw: u32, info: &TextureInfo) -> Result<Texture, Error> {
        self.thread();
        let id = self.res.textures.insert(TextureObject {
            raw: std::num::NonZeroU32::new(raw).map(glow::NativeTexture),
            target: target(info.dimension, info.samples),
            format: info.format,
            own: false,
            image: Cell::new(0),
        });
        self.res.table.textures.set(id, *info);
        self.res.table.residency.exempt_texture(id);
        Ok(id)
    }

    pub fn raw_texture(&self, texture: Texture) -> u32 {
        self.thread();
        self.res.table.textures.get(texture);
        self.res.textures.get(texture).raw.map_or(0, |t| t.0.get())
    }

    pub fn bindless_storage_texture(&self, texture: Texture) -> u64 {
        self.thread();
        let info = self.res.table.textures.get(texture);
        self.caps.assert_image_slot(texture);
        let t = self.res.textures.get(texture);
        if t.image.get() == 0 {
            let b = self.extra.bindless.as_ref().unwrap();
            let raw = t.raw.map_or(0, |r| r.0.get());
            let layered = info.layers > 1 || info.dimension == TextureDimension::D3;
            let internal = t.format.gl()[0];
            let root = self
                .res
                .table
                .parents
                .find(texture)
                .copied()
                .unwrap_or(texture);
            unsafe {
                let handle = (b.handle)(raw, 0, layered as u8, 0, internal);
                if self.res.table.residency.texture(root) > 0 {
                    (b.resident)(handle, glow::READ_WRITE);
                }
                t.image.set(handle);
            }
        }
        t.image.get()
    }

    pub fn format_usage(&self, format: Format) -> TextureUsage {
        self.thread();
        let s3tc = self.has(99, "GL_EXT_texture_compression_s3tc");
        let present = match format {
            Format::Bc1Unorm | Format::Bc2Unorm | Format::Bc3Unorm => s3tc,
            Format::Bc1Srgb | Format::Bc2Srgb | Format::Bc3Srgb => {
                s3tc && self.has(99, "GL_EXT_texture_sRGB")
            }
            f if (Format::Bc6hUfloat..=Format::Bc7Srgb).contains(&f) => {
                self.has(42, "GL_ARB_texture_compression_bptc")
            }
            f if (Format::Etc2Rgb8Unorm..=Format::EacRg11Snorm).contains(&f) => {
                self.has(43, "GL_ARB_ES3_compatibility")
            }
            f if (Format::Astc4x4Unorm..=Format::Astc12x12Float).contains(&f) => {
                self.has(99, "GL_KHR_texture_compression_astc_ldr")
            }
            Format::B5g6r5Unorm => self.has(41, "GL_ARB_ES2_compatibility"),
            Format::S8Uint => self.has(44, "GL_ARB_texture_stencil8"),
            _ => true,
        };
        let [internal, kind, _] = format.gl();
        if !present || internal == 0 {
            return TextureUsage::default();
        }
        let views = self.has(43, "GL_ARB_texture_view") && self.has(42, "GL_ARB_texture_storage");
        let base = TextureUsage::SAMPLED
            | if views {
                TextureUsage::VIEW
            } else {
                TextureUsage::default()
            };
        let copies = TextureUsage::COPY_SRC | TextureUsage::COPY_DST;
        let readable = self.extra.compressed_sub.is_some() && self.extra.copy_image.is_some();
        match kind {
            0 if readable => base | copies | TextureUsage::HOST,
            0 => base | TextureUsage::COPY_DST | TextureUsage::HOST,
            glow::DEPTH_STENCIL if format == Format::D24UnormS8 => base | TextureUsage::DEPTH,
            glow::DEPTH_COMPONENT | glow::DEPTH_STENCIL | glow::STENCIL_INDEX => {
                base | TextureUsage::DEPTH | copies
            }
            _ => {
                let unrenderable = matches!(
                    format,
                    Format::R8Snorm
                        | Format::Rg8Snorm
                        | Format::Rgba8Snorm
                        | Format::R16Snorm
                        | Format::Rg16Snorm
                        | Format::Rgba16Snorm
                        | Format::Rgb9e5Float
                        | Format::Rgb32Uint
                        | Format::Rgb32Sint
                        | Format::Rgb32Float
                );
                let unstorable = matches!(
                    format,
                    Format::Rgba8Srgb
                        | Format::Bgra8Srgb
                        | Format::B5g6r5Unorm
                        | Format::Rgb9e5Float
                        | Format::Rgb32Uint
                        | Format::Rgb32Sint
                        | Format::Rgb32Float
                );
                let mut u = base | TextureUsage::COPY_DST | TextureUsage::HOST;
                if !unrenderable {
                    u |= TextureUsage::COLOR | TextureUsage::COPY_SRC;
                }
                if !unstorable && self.has(43, "GL_ARB_shader_storage_buffer_object") {
                    u |= TextureUsage::STORAGE;
                    if matches!(format, Format::R32Uint | Format::R32Sint) {
                        u |= TextureUsage::ATOMIC;
                    }
                }
                u
            }
        }
    }

    pub fn format_samples(&self, format: Format, usage: TextureUsage) -> u32 {
        self.thread();
        if !self.format_usage(format).contains(usage) {
            return 0;
        }
        let [depth, integer, color, image] = self.samples;
        let mut max = match format.gl()[1] {
            0 => 1,
            glow::DEPTH_COMPONENT | glow::DEPTH_STENCIL | glow::STENCIL_INDEX => depth,
            _ if format.uint() || format.sint() => integer,
            _ => color,
        };
        if usage.contains(TextureUsage::STORAGE) {
            max = max.min(image);
        }
        (0..7)
            .map(|i| 1 << i)
            .filter(|&n| n <= max.max(1))
            .fold(0, |m, n| m | n as u32)
    }

    pub fn create_texture_view(
        &mut self,
        texture: Texture,
        info: &TextureViewInfo,
    ) -> Result<Texture, Error> {
        self.thread();
        let info = &self.res.table.resolve_view(texture, info);
        self.caps.check_view(info.dimension)?;
        let view = self
            .extra
            .view
            .ok_or_else(|| Error::Unsupported("texture views need GL 4.3".into()))?;
        let parent = self.res.textures.get(texture);
        let usable = self.format_usage(info.format);
        let counts = self.format_samples(info.format, info.usage);
        caps::check_support(
            "create_texture_view",
            info.format,
            info.usage | TextureUsage::VIEW,
            1,
            usable,
            counts,
        )?;
        let view = TextureObject::view(&self.gl, &self.extra, view, parent, info)?;
        let id = self.res.textures.insert(view);
        self.res.table.check_view(id, texture, info);
        Ok(id)
    }

    pub fn destroy_texture(&mut self, texture: Texture) {
        self.thread();
        self.res.table.drop_texture(texture);
        self.res.textures.remove(texture).delete(&self.gl);
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<Sampler, Error> {
        self.thread();
        self.caps.check_sampler(info)?;
        let raw = sampler::create(&self.gl, info)?;
        let id = self.res.samplers.insert(raw);
        self.res.table.samplers.set(id, ());
        Ok(id)
    }

    pub fn destroy_sampler(&mut self, sampler: Sampler) {
        self.thread();
        self.res.table.samplers.clear(sampler);
        sampler::delete(&self.gl, self.res.samplers.remove(sampler));
    }

    pub fn create_query_pool(
        &mut self,
        kind: crate::QueryKind,
        count: u32,
    ) -> Result<QueryPool, Error> {
        self.thread();
        self.caps.check_query_pool(kind, count)?;
        let id = self.res.queries.insert(Query::new(&self.gl, kind, count)?);
        self.res.table.queries.set(id, (kind, count));
        Ok(id)
    }

    pub fn destroy_query_pool(&mut self, heap: QueryPool) {
        self.thread();
        self.res.table.drop_query(heap);
        self.res.queries.remove(heap).delete(&self.gl);
    }

    pub fn create_timeline(&mut self) -> Result<Timeline, Error> {
        self.thread();
        let fences = Rc::new(RefCell::new(Fences {
            fences: Vec::new(),
            value: 0,
        }));
        let id = self.res.timelines.insert(fences.clone());
        Ok(Timeline {
            id,
            token: self.res.table.add_timeline(id),
            gl: self.gl.clone(),
            fences,
            reset: self.extra.reset,
            thread: self.thread,
        })
    }

    pub fn destroy_timeline(&mut self, timeline: Timeline) {
        self.thread();
        self.res.table.drop_timeline(&timeline);
        let fences = self.res.timelines.remove(timeline.id);
        fences.borrow().delete(&self.gl);
    }

    fn healthy(&self) -> Result<(), Error> {
        match self.extra.reset.map(|f| unsafe { f() }) {
            Some(0) | None => Ok(()),
            Some(status) => Err(Error::DeviceLost(format!("GL context lost ({status:#x})"))),
        }
    }

    pub fn create_command_pool(&mut self, queue: usize) -> Result<CommandPool, Error> {
        self.thread();
        check_eq!(queue, 0, "GL has one queue");
        let pool = self.res.pools.insert(());
        self.res.table.pools.set(pool, (queue, Cell::new(false)));
        Ok(pool)
    }

    pub fn reset_command_pool(&mut self, pool: CommandPool) -> Result<(), Error> {
        self.thread();
        self.res.table.pools.get(pool);
        Ok(())
    }

    pub fn destroy_command_pool(&mut self, pool: CommandPool) {
        self.thread();
        self.res.table.pools.clear(pool);
        self.res.pools.remove(pool);
    }

    fn recording(
        &self,
        pool: CommandPool,
        base: &'static str,
        samples: u32,
    ) -> Result<Recording<'_>, Error> {
        self.thread();
        check!(
            !self.open.get(),
            "GL runs commands as they are recorded; submit, finish or drop the open command buffer first"
        );
        let check = Check::new(&self.caps, &self.res.table, pool, base, samples);
        if let Some(b) = self
            .res
            .buffers
            .values()
            .find(|b| !b.persistent && !b.data.get().is_null())
        {
            let _ = b;
            return Err(Error::Invalid(
                "a buffer is mapped; unmap it before recording".into(),
            ));
        }
        self.check_host_programs()?;
        self.check_host_queries()?;
        self.healthy()?;
        let host = crate::gl::errors::drain(&self.gl);
        let state = State::save(
            &self.gl,
            &self.extra,
            &self.caps,
            &|v, e| self.has(v, e),
            self.units.max(1),
        );
        crate::gl::errors::raise(&self.gl, &self.extra, &host);
        if base == "outside" {
            state.clear(&self.gl, &self.extra);
        }
        let recorder = Recorder::new(
            &self.gl,
            &self.extra,
            &self.res,
            &self.caps,
            (self.fbo, self.read),
            state,
            &self.open,
        );
        self.open.set(true);
        Ok(Recording { check, recorder })
    }

    pub fn begin_commands(&self, pool: CommandPool) -> Result<CommandBuffer<'_>, Error> {
        let mut recording = self.recording(pool, "outside", 0)?;
        recording.recorder.begin()?;
        Ok(CommandBuffer { recording })
    }

    pub fn import_render_pass(
        &self,
        pool: CommandPool,
        info: &ImportedPassInfo,
    ) -> Result<RenderPass<'_, '_>, Error> {
        let mut recording = self.recording(pool, "render", info.samples)?;
        recording.recorder.import(info.width, info.height);
        Ok(RenderPass {
            recording: Held::Owned(Box::new(recording)),
        })
    }

    pub fn submit<'d>(
        &self,
        commands: impl IntoIterator<Item = CommandBuffer<'d>>,
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) -> Result<(), Error> {
        self.thread();
        let table = &self.res.table;
        table.check_timelines(waits);
        table.check_timelines(signals);
        let mut queue = None;
        for mut c in commands {
            let recording = &mut c.recording;
            recording.check.finish();
            let q = recording.check.queue();
            check!(
                queue.is_none_or(|p| p == q),
                "submit mixes command buffers of queues {} and {q}; submit each queue on its own",
                queue.unwrap_or(q)
            );
            queue = Some(q);
            recording.recorder.finish()?;
        }
        let queue = queue.expect("submit needs at least one command buffer");
        table.check_signals(queue, signals);
        for &(t, value) in waits {
            let e = t.fences.borrow();
            if e.value >= value {
                continue;
            }
            let &(_, fence) = e
                .fences
                .iter()
                .find(|(v, _)| *v >= value)
                .expect("wait on a timeline value no submit signals");
            unsafe { self.gl.wait_sync(fence, 0, glow::TIMEOUT_IGNORED) };
        }
        for &(t, value) in signals {
            t.fences.borrow_mut().signal(&self.gl, value)?;
        }
        table.record_signals(queue, signals);
        unsafe { self.gl.flush() };
        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.thread();
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
            for t in self.res.timelines.drain() {
                t.borrow().delete(gl);
            }
            gl.delete_framebuffer(self.fbo);
            gl.delete_framebuffer(self.read);
        }
    }
}
