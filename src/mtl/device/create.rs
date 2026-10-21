use super::Device;
use crate::{Atomic64, DeviceInfo, Error};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4Compiler, MTL4CompilerDescriptor, MTLCreateSystemDefaultDevice, MTLDevice};
use std::sync::Mutex;

struct Shared(Option<Retained<ProtocolObject<dyn MTL4Compiler>>>);

unsafe impl Send for Shared {}

static COMPILER: Mutex<Shared> = Mutex::new(Shared(None));

fn compiler(
    raw: &ProtocolObject<dyn MTLDevice>,
) -> Result<Retained<ProtocolObject<dyn MTL4Compiler>>, Error> {
    let mut shared = COMPILER.lock().unwrap();
    if let Some(compiler) = &shared.0 {
        return Ok(compiler.clone());
    }
    let compiler = raw
        .newCompilerWithDescriptor_error(&MTL4CompilerDescriptor::new())
        .map_err(Error::from)?;
    shared.0 = Some(compiler.clone());
    Ok(compiler)
}

fn supported(raw: &ProtocolObject<dyn MTLDevice>) -> DeviceInfo {
    let caps = Device::detect(raw, 1, 0);
    DeviceInfo {
        queue_count: 32,
        buffer_addresses: true,
        timelines: true,
        partially_bound: true,
        update_after_bind: true,
        standard_layout: true,
        synchronization2: true,
        dynamic_rendering: true,
        sampler_anisotropy: true,
        cube_arrays: true,
        multi_viewports: true,
        depth_clamp: true,
        precise_occlusion: true,
        line_fill: true,
        multi_draw_indirect: true,
        bias_clamp: true,
        independent_blend: true,
        dual_source_blend: true,
        shader_int64: true,
        unformatted_reads: true,
        unformatted_writes: true,
        draw_parameters: true,
        runtime_descriptor_arrays: true,
        nonuniform_indexing: true,
        indirect_count: true,
        buffer_atomics64: caps.atomics64.contains(Atomic64::BUFFER_ALL),
        shared_atomics64: caps.atomics64.contains(Atomic64::SHARED),
        acquire_release_atomics: true,
        mirror_clamp: true,
        shader_viewport_index: true,
        subgroup_size_control: true,
        full_subgroups: caps.full_subgroups,
        subgroup_rotate: true,
        dynamic_indexing: true,
        fragment_stores: true,
        vertex_stores: true,
        mesh_shaders: caps.mesh_shaders,
        object_shaders: caps.object_shaders,
        host_texture_reads: caps.host_texture_reads,
        attachment_reads: true,
        ordered_reads: true,
        image_atomics64: caps.atomics64.contains(Atomic64::IMAGE),
        swapchain: true,
        present_wait: true,
        shared_textures: false,
    }
}

fn system() -> Result<Retained<ProtocolObject<dyn MTLDevice>>, Error> {
    MTLCreateSystemDefaultDevice().ok_or_else(|| Error::Unsupported("no Metal device".into()))
}

impl Device {
    pub fn supported_features() -> Result<DeviceInfo, Error> {
        let raw = system()?;
        Ok(supported(&raw))
    }

    pub fn create_device(info: &DeviceInfo) -> Result<Self, Error> {
        let raw = system()?;
        info.assert_usable(&supported(&raw));
        let compiler = compiler(&raw)?;
        Self::import_device(raw, compiler, info.queue_count)
    }
}
