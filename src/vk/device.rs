use crate::vk::accel::Accel;
use crate::vk::bindings;
use crate::vk::bindless::Bindless;
use crate::vk::buffer::Buffer;
use crate::vk::extensions::Extensions;
use crate::vk::ffi;
use crate::vk::generated::Generated;
use crate::vk::heap::Heap;
use crate::vk::queue::{Queue, timeline};
use crate::vk::resources::Resources;
use crate::vk::surface::Surface;
use crate::vk::texture::{Texture, aspect};
use crate::{
    AccelInfo, Caps, ColorTarget, Commands, ComputeInfo, DepthInfo, LayoutInfo, MeshInfo, RayInfo,
    RenderInfo, SamplerInfo, SparseRange, SparseRegion, SurfaceInfo, TextureInfo, ViewInfo,
    address, border, dimension, fill, filter, format, index, memory, query, token, topology, usage,
};
use ash::ext::debug_utils;
use ash::ext::{conditional_rendering, external_memory_host, mesh_shader, mutable_descriptor_type};
use ash::khr::push_descriptor;
use ash::khr::{
    acceleration_structure, deferred_host_operations, external_memory_fd, fragment_shading_rate,
    maintenance5, portability_enumeration, portability_subset, ray_query, ray_tracing_pipeline,
    surface, swapchain,
};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, CString};
use std::io::Cursor;

pub struct Device {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical: vk::PhysicalDevice,
    pub raw: ash::Device,
    ext: Extensions,
    owned: bool,
    memory: vk::PhysicalDeviceMemoryProperties,
    families: Vec<u32>,
    cache: vk::PipelineCache,
    bindless: Bindless,
    slots: vk::DescriptorSetLayout,
    layout: vk::PipelineLayout,
    res: Resources,
    queues: Vec<Queue>,
}

const FDS: &str = "VK_KHR_external_memory_fd is not enabled";

unsafe fn chained<'a, T>(
    features: &'a vk::PhysicalDeviceFeatures2,
    kind: vk::StructureType,
) -> Option<&'a T> {
    let mut next = features.p_next as *const vk::BaseInStructure;
    while let Some(s) = unsafe { next.as_ref() } {
        if s.s_type == kind {
            return Some(unsafe { &*(next as *const T) });
        }
        next = s.p_next;
    }
    None
}

fn image_usage(usage: u32) -> vk::ImageUsageFlags {
    let map = [
        (usage::SAMPLED, vk::ImageUsageFlags::SAMPLED),
        (usage::STORAGE, vk::ImageUsageFlags::STORAGE),
        (usage::COLOR, vk::ImageUsageFlags::COLOR_ATTACHMENT),
        (usage::DEPTH, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT),
        (usage::COPY_SRC, vk::ImageUsageFlags::TRANSFER_SRC),
        (usage::COPY_DST, vk::ImageUsageFlags::TRANSFER_DST),
    ];
    let mut bits = vk::ImageUsageFlags::empty();
    for (u, f) in map {
        if usage & u != 0 {
            bits |= f;
        }
    }
    bits
}

fn image_info(info: &TextureInfo) -> vk::ImageCreateInfo<'static> {
    let kind = match info.dimension {
        dimension::D1 | dimension::D1_ARRAY => vk::ImageType::TYPE_1D,
        dimension::D3 => vk::ImageType::TYPE_3D,
        dimension::D2 | dimension::D2_ARRAY | dimension::CUBE | dimension::CUBE_ARRAY => {
            vk::ImageType::TYPE_2D
        }
        d => panic!("unknown dimension {d}"),
    };
    let mut flags = vk::ImageCreateFlags::empty();
    if matches!(info.dimension, dimension::CUBE | dimension::CUBE_ARRAY) {
        flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
    }
    if info.usage & usage::VIEW != 0 {
        flags |= vk::ImageCreateFlags::MUTABLE_FORMAT;
    }
    let mut bits = image_usage(info.usage);
    if info.memory == memory::TRANSIENT {
        bits |= vk::ImageUsageFlags::TRANSIENT_ATTACHMENT;
    }
    vk::ImageCreateInfo::default()
        .flags(flags)
        .image_type(kind)
        .format(format::vk(info.format))
        .extent(vk::Extent3D {
            width: info.width,
            height: info.height,
            depth: info.depth,
        })
        .mip_levels(info.mips)
        .array_layers(info.layers)
        .samples(vk::SampleCountFlags::from_raw(info.samples))
        .usage(bits)
}

fn whole(info: &TextureInfo) -> ViewInfo {
    ViewInfo {
        dimension: info.dimension,
        format: info.format,
        base_mip: 0,
        mips: vk::REMAINING_MIP_LEVELS,
        base_layer: 0,
        layers: vk::REMAINING_ARRAY_LAYERS,
        usage: info.usage,
    }
}

fn has(list: &[vk::ExtensionProperties], name: &CStr) -> bool {
    list.iter().any(|e| e.extension_name_as_c_str() == Ok(name))
}

impl Device {
    pub fn new(queues: usize) -> Result<Self, String> {
        unsafe {
            let entry = ash::Entry::load().map_err(|e| e.to_string())?;
            let available = entry
                .enumerate_instance_extension_properties(None)
                .map_err(|e| e.to_string())?;
            let mut extensions = Vec::new();
            let mut flags = vk::InstanceCreateFlags::empty();
            if has(&available, debug_utils::NAME) {
                extensions.push(debug_utils::NAME.as_ptr());
            }
            for name in [
                surface::NAME,
                c"VK_EXT_metal_surface",
                c"VK_KHR_win32_surface",
                c"VK_KHR_xlib_surface",
                c"VK_KHR_xcb_surface",
                c"VK_KHR_wayland_surface",
                c"VK_KHR_android_surface",
            ] {
                if has(&available, name) {
                    extensions.push(name.as_ptr());
                }
            }
            if has(&available, portability_enumeration::NAME) {
                extensions.push(portability_enumeration::NAME.as_ptr());
                flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
            }
            let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
            let info = vk::InstanceCreateInfo::default()
                .flags(flags)
                .application_info(&app)
                .enabled_extension_names(&extensions);
            let instance = entry
                .create_instance(&info, None)
                .map_err(|e| e.to_string())?;
            let physicals = instance
                .enumerate_physical_devices()
                .map_err(|e| e.to_string())?;
            let physical = *physicals
                .iter()
                .find(|&&p| {
                    instance.get_physical_device_properties(p).device_type
                        == vk::PhysicalDeviceType::DISCRETE_GPU
                })
                .or(physicals.first())
                .ok_or("no Vulkan device")?;
            let families = instance.get_physical_device_queue_family_properties(physical);
            let mut order: Vec<_> = (0..families.len() as u32)
                .filter(|&f| {
                    families[f as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::COMPUTE)
                })
                .collect();
            order.sort_by_key(|&f| {
                !families[f as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            });
            let slots: Vec<(u32, u32)> = order
                .iter()
                .flat_map(|&f| (0..families[f as usize].queue_count).map(move |i| (f, i)))
                .take(queues)
                .collect();
            if slots.len() < queues {
                return Err(format!("device has {} queues", slots.len()));
            }
            let priorities = vec![1.0; queues];
            let infos: Vec<_> = order
                .iter()
                .filter_map(|&f| {
                    let count = slots.iter().filter(|s| s.0 == f).count();
                    (count > 0).then(|| {
                        vk::DeviceQueueCreateInfo::default()
                            .queue_family_index(f)
                            .queue_priorities(&priorities[..count])
                    })
                })
                .collect();
            let available = instance
                .enumerate_device_extension_properties(physical)
                .map_err(|e| e.to_string())?;
            let mut extensions = vec![mutable_descriptor_type::NAME.as_ptr()];
            if has(&available, swapchain::NAME) {
                extensions.push(swapchain::NAME.as_ptr());
            }
            if has(&available, portability_subset::NAME) {
                extensions.push(portability_subset::NAME.as_ptr());
            }
            if has(&available, push_descriptor::NAME) {
                extensions.push(push_descriptor::NAME.as_ptr());
            }
            let mut supported_generated = ffi::Features {
                s_type: ffi::FEATURES,
                next: std::ptr::null_mut(),
                generated: 0,
                dynamic: 0,
            };
            let mut query = vk::PhysicalDeviceFeatures2 {
                p_next: (&raw mut supported_generated).cast(),
                ..Default::default()
            };
            instance.get_physical_device_features2(physical, &mut query);
            let generated = has(&available, c"VK_EXT_device_generated_commands")
                && has(&available, maintenance5::NAME)
                && supported_generated.generated != 0;
            let mut enabled_generated = ffi::Features {
                s_type: ffi::FEATURES,
                next: std::ptr::null_mut(),
                generated: 1,
                dynamic: 0,
            };
            let mut maintenance =
                vk::PhysicalDeviceMaintenance5FeaturesKHR::default().maintenance5(true);
            if generated {
                extensions.push(c"VK_EXT_device_generated_commands".as_ptr());
                extensions.push(maintenance5::NAME.as_ptr());
            }
            let mut supported_conditional =
                vk::PhysicalDeviceConditionalRenderingFeaturesEXT::default();
            let mut query =
                vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_conditional);
            instance.get_physical_device_features2(physical, &mut query);
            let conditional = has(&available, conditional_rendering::NAME)
                && supported_conditional.conditional_rendering != 0;
            let mut conditions = vk::PhysicalDeviceConditionalRenderingFeaturesEXT::default()
                .conditional_rendering(true);
            if conditional {
                extensions.push(conditional_rendering::NAME.as_ptr());
            }
            if has(&available, external_memory_fd::NAME) {
                extensions.push(external_memory_fd::NAME.as_ptr());
            }
            if has(&available, external_memory_host::NAME) {
                extensions.push(external_memory_host::NAME.as_ptr());
            }
            let mut supported_rate = vk::PhysicalDeviceFragmentShadingRateFeaturesKHR::default();
            let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_rate);
            instance.get_physical_device_features2(physical, &mut query);
            let rated = has(&available, fragment_shading_rate::NAME)
                && supported_rate.pipeline_fragment_shading_rate != 0;
            let mut rate = vk::PhysicalDeviceFragmentShadingRateFeaturesKHR::default()
                .pipeline_fragment_shading_rate(true)
                .primitive_fragment_shading_rate(
                    supported_rate.primitive_fragment_shading_rate != 0,
                )
                .attachment_fragment_shading_rate(
                    supported_rate.attachment_fragment_shading_rate != 0,
                );
            if rated {
                extensions.push(fragment_shading_rate::NAME.as_ptr());
            }
            let mut supported_mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default();
            let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_mesh);
            instance.get_physical_device_features2(physical, &mut query);
            let mut mesh = vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
                .task_shader(supported_mesh.task_shader != 0)
                .mesh_shader(supported_mesh.mesh_shader != 0);
            let meshes = has(&available, mesh_shader::NAME) && supported_mesh.mesh_shader != 0;
            let mut supported_accel = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();
            let mut supported_query = vk::PhysicalDeviceRayQueryFeaturesKHR::default();
            let mut supported_pipeline = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();
            let mut query = vk::PhysicalDeviceFeatures2::default()
                .push_next(&mut supported_accel)
                .push_next(&mut supported_query)
                .push_next(&mut supported_pipeline);
            instance.get_physical_device_features2(physical, &mut query);
            let rays = has(&available, acceleration_structure::NAME)
                && has(&available, deferred_host_operations::NAME)
                && has(&available, ray_query::NAME)
                && supported_accel.acceleration_structure != 0
                && supported_query.ray_query != 0;
            let mut accel = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
                .acceleration_structure(true);
            let mut rq = vk::PhysicalDeviceRayQueryFeaturesKHR::default().ray_query(true);
            let pipelines = rays
                && has(&available, ray_tracing_pipeline::NAME)
                && supported_pipeline.ray_tracing_pipeline != 0;
            let mut rt = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default()
                .ray_tracing_pipeline(true);
            if rays {
                extensions.push(acceleration_structure::NAME.as_ptr());
                extensions.push(deferred_host_operations::NAME.as_ptr());
                extensions.push(ray_query::NAME.as_ptr());
            }
            if pipelines {
                extensions.push(ray_tracing_pipeline::NAME.as_ptr());
            }
            if meshes {
                extensions.push(mesh_shader::NAME.as_ptr());
            }
            let supported = instance.get_physical_device_features(physical);
            let features = vk::PhysicalDeviceFeatures::default()
                .sampler_anisotropy(supported.sampler_anisotropy != 0)
                .multi_viewport(supported.multi_viewport != 0)
                .depth_bounds(supported.depth_bounds != 0)
                .pipeline_statistics_query(supported.pipeline_statistics_query != 0)
                .depth_clamp(supported.depth_clamp != 0)
                .sparse_binding(supported.sparse_binding != 0)
                .sparse_residency_buffer(supported.sparse_residency_buffer != 0)
                .sparse_residency_image2_d(supported.sparse_residency_image2_d != 0)
                .shader_int64(supported.shader_int64 != 0)
                .occlusion_query_precise(supported.occlusion_query_precise != 0)
                .fill_mode_non_solid(supported.fill_mode_non_solid != 0)
                .multi_draw_indirect(supported.multi_draw_indirect != 0)
                .depth_bias_clamp(supported.depth_bias_clamp != 0)
                .independent_blend(supported.independent_blend != 0)
                .dual_src_blend(supported.dual_src_blend != 0)
                .shader_storage_image_read_without_format(
                    supported.shader_storage_image_read_without_format != 0,
                )
                .shader_storage_image_write_without_format(
                    supported.shader_storage_image_write_without_format != 0,
                );
            let mut v11 = vk::PhysicalDeviceVulkan11Features::default()
                .multiview(true)
                .shader_draw_parameters(true);
            let mut supported_v12 = vk::PhysicalDeviceVulkan12Features::default();
            let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_v12);
            instance.get_physical_device_features2(physical, &mut query);
            let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
                .buffer_device_address(true)
                .timeline_semaphore(true)
                .descriptor_indexing(true)
                .runtime_descriptor_array(true)
                .descriptor_binding_partially_bound(true)
                .descriptor_binding_sampled_image_update_after_bind(true)
                .descriptor_binding_storage_image_update_after_bind(true)
                .shader_sampled_image_array_non_uniform_indexing(true)
                .shader_storage_image_array_non_uniform_indexing(true)
                .descriptor_binding_storage_buffer_update_after_bind(
                    supported_v12.descriptor_binding_storage_buffer_update_after_bind != 0,
                )
                .shader_storage_buffer_array_non_uniform_indexing(
                    supported_v12.shader_storage_buffer_array_non_uniform_indexing != 0,
                )
                .shader_output_viewport_index(supported_v12.shader_output_viewport_index != 0)
                .shader_output_layer(supported_v12.shader_output_layer != 0);
            let mut mutable = vk::PhysicalDeviceMutableDescriptorTypeFeaturesEXT::default()
                .mutable_descriptor_type(true);
            let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
                .synchronization2(true)
                .dynamic_rendering(true)
                .maintenance4(true);
            let info = vk::DeviceCreateInfo::default()
                .queue_create_infos(&infos)
                .enabled_extension_names(&extensions)
                .enabled_features(&features)
                .push_next(&mut v11)
                .push_next(&mut v12)
                .push_next(&mut v13)
                .push_next(&mut mutable);
            let info = if meshes {
                info.push_next(&mut mesh)
            } else {
                info
            };
            let info = if rays {
                info.push_next(&mut accel).push_next(&mut rq)
            } else {
                info
            };
            let mut info = if pipelines {
                info.push_next(&mut rt)
            } else {
                info
            };
            if rated {
                info = info.push_next(&mut rate);
            }
            if conditional {
                info = info.push_next(&mut conditions);
            }
            if generated {
                info = info.push_next(&mut maintenance);
                enabled_generated.next = info.p_next as *mut _;
                info.p_next = (&raw const enabled_generated).cast();
            }
            let device = instance
                .create_device(physical, &info, None)
                .map_err(|e| e.to_string())?;
            let mut enabled = vk::PhysicalDeviceFeatures2::default().features(features);
            enabled.p_next = info.p_next as *mut _;
            match Self::adopt(
                entry,
                instance.clone(),
                physical,
                device.clone(),
                &slots,
                &enabled,
            ) {
                Ok(mut d) => {
                    d.owned = true;
                    Ok(d)
                }
                Err(e) => {
                    device.destroy_device(None);
                    instance.destroy_instance(None);
                    Err(e)
                }
            }
        }
    }

    pub fn adopt(
        entry: ash::Entry,
        instance: ash::Instance,
        physical: vk::PhysicalDevice,
        raw: ash::Device,
        queues: &[(u32, u32)],
        features: &vk::PhysicalDeviceFeatures2,
    ) -> Result<Self, String> {
        unsafe {
            let name = c"vkCmdBeginDebugUtilsLabelEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let debug = loaded.map(|_| debug_utils::Device::new(&instance, &raw));
            let name = c"vkCmdDrawMeshTasksEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let mesh = loaded.map(|_| mesh_shader::Device::new(&instance, &raw));
            let name = c"vkCmdBuildAccelerationStructuresKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let ray = loaded.map(|_| acceleration_structure::Device::new(&instance, &raw));
            let generated = Generated::load(&instance, &raw);
            let name = c"vkCreateRayTracingPipelinesKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let rays = loaded.map(|_| ray_tracing_pipeline::Device::new(&instance, &raw));
            let name = c"vkGetMemoryHostPointerPropertiesEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let host = loaded.map(|_| external_memory_host::Device::new(&instance, &raw));
            let name = c"vkCmdBeginConditionalRenderingEXT";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let conditional = loaded.map(|_| conditional_rendering::Device::new(&instance, &raw));
            let name = c"vkGetMemoryFdKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let fds = loaded.map(|_| external_memory_fd::Device::new(&instance, &raw));
            let name = c"vkCmdSetFragmentShadingRateKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let rates = loaded.map(|_| fragment_shading_rate::Device::new(&instance, &raw));
            let name = c"vkCreateSwapchainKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let surfaces = loaded.map(|_| {
                (
                    surface::Instance::new(&entry, &instance),
                    swapchain::Device::new(&instance, &raw),
                )
            });
            let name = c"vkCmdPushDescriptorSetKHR";
            let loaded = (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr());
            let push = loaded.map(|_| push_descriptor::Device::new(&instance, &raw));
            let limits = instance.get_physical_device_properties(physical).limits;
            let range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::ALL)
                .size(limits.max_push_constants_size.min(256));
            let mut properties12 = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties =
                vk::PhysicalDeviceProperties2::default().push_next(&mut properties12);
            instance.get_physical_device_properties2(physical, &mut properties);
            let slots = bindings::layout(&raw, push.is_some()).map_err(|e| e.to_string())?;
            let v12 = chained::<vk::PhysicalDeviceVulkan12Features>(
                features,
                vk::StructureType::PHYSICAL_DEVICE_VULKAN_1_2_FEATURES,
            );
            let buffers =
                v12.is_some_and(|f| f.descriptor_binding_storage_buffer_update_after_bind != 0);
            let bindless = match Bindless::new(&raw, &properties12, buffers) {
                Ok(b) => b,
                Err(e) => {
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e.to_string());
                }
            };
            let ranges = [range];
            let sets = [slots, bindless.layout];
            let info = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&sets)
                .push_constant_ranges(&ranges);
            let layout = match raw.create_pipeline_layout(&info, None) {
                Ok(l) => l,
                Err(e) => {
                    bindless.destroy(&raw);
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e.to_string());
                }
            };
            let mut families: Vec<u32> = queues.iter().map(|q| q.0).collect();
            families.sort();
            families.dedup();
            let queues = match queues
                .iter()
                .map(|&(f, i)| Queue::new(&raw, f, i))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(q) => q,
                Err(e) => {
                    raw.destroy_pipeline_layout(layout, None);
                    bindless.destroy(&raw);
                    raw.destroy_descriptor_set_layout(slots, None);
                    return Err(e);
                }
            };
            Ok(Self {
                memory: instance.get_physical_device_memory_properties(physical),
                families,
                cache: vk::PipelineCache::null(),
                entry,
                instance,
                physical,
                raw,
                ext: Extensions {
                    debug,
                    mesh,
                    ray,
                    rays,
                    rates,
                    host,
                    fds,
                    conditional,
                    generated,
                    surfaces,
                    features: features.features,
                    push,
                },
                slots,
                owned: false,
                bindless,
                layout,
                res: Resources::default(),
                queues,
            })
        }
    }

    fn memory_type(&self, bits: u32, memory: u32) -> Result<u32, String> {
        let host = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let (flags, name) = match memory {
            memory::DEVICE => (vk::MemoryPropertyFlags::DEVICE_LOCAL, "device local"),
            memory::UPLOAD => (host, "host coherent"),
            memory::READBACK => (host | vk::MemoryPropertyFlags::HOST_CACHED, "host cached"),
            memory::TRANSIENT => (
                vk::MemoryPropertyFlags::DEVICE_LOCAL | vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
                "lazily allocated",
            ),
            _ => panic!("unknown memory {memory}"),
        };
        self.find_type(bits, flags)
            .ok_or(format!("no {name} memory type fits this resource"))
    }

    fn find_type(&self, bits: u32, flags: vk::MemoryPropertyFlags) -> Option<u32> {
        let types = &self.memory.memory_types[..self.memory.memory_type_count as usize];
        (0..types.len() as u32)
            .find(|&i| bits & (1 << i) != 0 && types[i as usize].property_flags.contains(flags))
    }

    pub fn adopt_raw(
        proc: u64,
        instance: u64,
        physical: u64,
        device: u64,
        queues: &[(u32, u32)],
        features: u64,
    ) -> Result<Self, String> {
        unsafe {
            let none = vk::PhysicalDeviceFeatures2::default();
            let features = (features as *const vk::PhysicalDeviceFeatures2)
                .as_ref()
                .unwrap_or(&none);
            let get = std::mem::transmute::<usize, vk::PFN_vkGetInstanceProcAddr>(proc as usize);
            let entry = ash::Entry::from_static_fn(ash::StaticFn {
                get_instance_proc_addr: get,
            });
            let instance = ash::Instance::load(entry.static_fn(), vk::Handle::from_raw(instance));
            let raw = ash::Device::load(instance.fp_v1_0(), vk::Handle::from_raw(device));
            Self::adopt(
                entry,
                instance,
                vk::Handle::from_raw(physical),
                raw,
                queues,
                features,
            )
        }
    }

    pub fn caps(&self) -> Caps {
        let mut host = vk::PhysicalDeviceExternalMemoryHostPropertiesEXT::default();
        if self.ext.host.is_some() {
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut host);
            unsafe {
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties)
            };
        }
        let mut rays = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
        if self.ext.rays.is_some() {
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut rays);
            unsafe {
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties)
            };
        }
        let features = &self.ext.features;
        let (limits, families) = unsafe {
            (
                self.instance
                    .get_physical_device_properties(self.physical)
                    .limits,
                self.instance
                    .get_physical_device_queue_family_properties(self.physical),
            )
        };
        let sparse = features.sparse_binding != 0
            && features.sparse_residency_buffer != 0
            && self.queues.iter().all(|q| {
                families[q.family as usize]
                    .queue_flags
                    .contains(vk::QueueFlags::SPARSE_BINDING)
            });
        Caps {
            mesh: self.ext.mesh.is_some(),
            push: limits.max_push_constants_size.min(256),
            queues: self.queues.len() as u32,
            samplers: self.bindless.samplers,
            textures: self.bindless.images / 2,
            tick: limits.timestamp_period,
            timestamps: self
                .queues
                .iter()
                .all(|q| families[q.family as usize].timestamp_valid_bits > 0),
            precise: features.occlusion_query_precise != 0,
            ray: self.ext.ray.is_some(),
            generated: self.ext.generated.is_some(),
            handle_size: rays.shader_group_handle_size,
            handle_alignment: rays.shader_group_handle_alignment,
            table_alignment: rays.shader_group_base_alignment,
            shading_rate: self.ext.rates.is_some(),
            host_memory: self.ext.host.is_some(),
            shared_memory: self.ext.fds.is_some(),
            depth_bounds: features.depth_bounds != 0,
            conditional: self.ext.conditional.is_some(),
            statistics: features.pipeline_statistics_query != 0,
            viewports: if features.multi_viewport != 0 {
                limits.max_viewports
            } else {
                1
            },
            host_alignment: host.min_imported_host_pointer_alignment as u32,
            sparse,
            instance: 64,
            views: unsafe {
                let mut multiview = vk::PhysicalDeviceMultiviewProperties::default();
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut multiview);
                self.instance
                    .get_physical_device_properties2(self.physical, &mut properties);
                multiview.max_multiview_view_count
            },
            memoryless: self.memory.memory_types[..self.memory.memory_type_count as usize]
                .iter()
                .any(|t| {
                    t.property_flags
                        .contains(vk::MemoryPropertyFlags::LAZILY_ALLOCATED)
                }),
            graphics: self
                .queues
                .iter()
                .enumerate()
                .filter(|(_, q)| {
                    families[q.family as usize]
                        .queue_flags
                        .contains(vk::QueueFlags::GRAPHICS)
                })
                .fold(0, |mask, (i, _)| mask | 1 << i),
            labels: self.ext.debug.is_some(),
            surfaces: self.ext.surfaces.is_some(),
            rays: self.ext.rays.is_some(),
            anisotropy: features.sampler_anisotropy != 0,
            depth_clamp: features.depth_clamp != 0,
            line_fill: features.fill_mode_non_solid != 0,
            multi_draw: features.multi_draw_indirect != 0,
            sparse_textures: sparse && features.sparse_residency_image2_d != 0,
            buffers: self.bindless.buffers,
            compute: true,
            slots: self.ext.push.is_some(),
            storage: self.ext.push.is_some(),
            persistent: true,
            alignment: limits
                .min_uniform_buffer_offset_alignment
                .max(limits.min_storage_buffer_offset_alignment) as u32,
        }
    }

    pub fn name(&self) -> String {
        let properties = unsafe { self.instance.get_physical_device_properties(self.physical) };
        properties
            .device_name_as_c_str()
            .map_or(String::new(), |n| n.to_string_lossy().into_owned())
    }

    pub fn create_buffer(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        let buffer = self.allocate(size, memory, self.usage())?;
        Ok(self.insert_buffer(buffer))
    }

    fn buffer_info<'a>(
        &'a self,
        size: u64,
        usage: vk::BufferUsageFlags,
        wide: &'a mut vk::BufferUsageFlags2CreateInfoKHR<'a>,
    ) -> vk::BufferCreateInfo<'a> {
        let mut info = vk::BufferCreateInfo::default().size(size).usage(usage);
        if self.families.len() > 1 {
            info = info
                .sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families);
        }
        if self.ext.generated.is_some() {
            wide.usage =
                vk::BufferUsageFlags2KHR::from_raw(usage.as_raw() as u64 | ffi::PREPROCESS);
            info = info.push_next(wide);
        }
        info
    }

    fn share_image<'a>(&'a self, info: vk::ImageCreateInfo<'a>) -> vk::ImageCreateInfo<'a> {
        if self.families.len() > 1 {
            info.sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&self.families)
        } else {
            info
        }
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        let mut usage = vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST
            | vk::BufferUsageFlags::UNIFORM_BUFFER
            | vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::INDEX_BUFFER
            | vk::BufferUsageFlags::VERTEX_BUFFER
            | vk::BufferUsageFlags::INDIRECT_BUFFER
            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
        if self.ext.ray.is_some() {
            usage |= vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR;
        }
        if self.ext.rays.is_some() {
            usage |= vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR;
        }
        if self.ext.conditional.is_some() {
            usage |= vk::BufferUsageFlags::CONDITIONAL_RENDERING_EXT;
        }
        usage
    }

    fn free_buffer(&self, buffer: &Buffer) {
        unsafe {
            if buffer.own {
                self.raw.destroy_buffer(buffer.raw, None);
            }
            if buffer.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(buffer.memory, None);
            }
        }
    }

    pub fn create_host_buffer(&mut self, pointer: *mut u8, size: u64) -> Result<u32, String> {
        let host = self
            .ext
            .host
            .as_ref()
            .ok_or("VK_EXT_external_memory_host is not enabled")?;
        let kind = vk::ExternalMemoryHandleTypeFlags::HOST_ALLOCATION_EXT;
        let mut properties = vk::MemoryHostPointerPropertiesEXT::default();
        let result = unsafe {
            (host.fp().get_memory_host_pointer_properties_ext)(
                self.raw.handle(),
                kind,
                pointer.cast(),
                &mut properties,
            )
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let mut external = vk::ExternalMemoryBufferCreateInfo::default().handle_types(kind);
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .push_next(&mut external);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let bits = needs.memory_type_bits & properties.memory_type_bits;
            let Some(index) = self.find_type(bits, vk::MemoryPropertyFlags::HOST_VISIBLE) else {
                self.raw.destroy_buffer(raw, None);
                return Err("no host visible memory type fits this pointer".into());
            };
            let mut import = vk::ImportMemoryHostPointerInfoEXT::default()
                .handle_type(kind)
                .host_pointer(pointer.cast());
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(size)
                .memory_type_index(index)
                .push_next(&mut import)
                .push_next(&mut flags);
            let memory = self
                .raw
                .allocate_memory(&alloc, None)
                .map_err(|e| e.to_string())?;
            self.raw
                .bind_buffer_memory(raw, memory, 0)
                .map_err(|e| e.to_string())?;
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(Buffer {
                raw,
                memory,
                address,
                data: pointer,
                own: true,
            }))
        }
    }

    pub fn create_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.heap(size, memory, false, None)
    }

    pub fn create_shared_heap(&mut self, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fds.as_ref().ok_or(FDS)?;
        self.heap(size, memory, true, None)
    }

    pub fn import_heap(&mut self, fd: i32, size: u64, memory: u32) -> Result<u32, String> {
        self.ext.fds.as_ref().ok_or(FDS)?;
        self.heap(size, memory, true, Some(fd))
    }

    pub fn export_heap(&self, heap: u32) -> Result<i32, String> {
        let fds = self.ext.fds.as_ref().ok_or(FDS)?;
        let info = vk::MemoryGetFdInfoKHR::default()
            .memory(self.res.heaps.get(heap).memory)
            .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        unsafe { fds.get_memory_fd(&info).map_err(|e| e.to_string()) }
    }

    fn heap(
        &mut self,
        size: u64,
        memory: u32,
        shared: bool,
        fd: Option<i32>,
    ) -> Result<u32, String> {
        let index = self.memory_type(!0, memory)?;
        let kind = vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD;
        let mut flags =
            vk::MemoryAllocateFlagsInfo::default().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
        let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(kind);
        let mut import = vk::ImportMemoryFdInfoKHR::default()
            .handle_type(kind)
            .fd(fd.unwrap_or(-1));
        let mut info = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut flags);
        if fd.is_some() {
            info = info.push_next(&mut import);
        } else if shared {
            info = info.push_next(&mut export);
        }
        unsafe {
            let memory = self
                .raw
                .allocate_memory(&info, None)
                .map_err(|e| e.to_string())?;
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            let data = if visible {
                self.raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                    .map_err(|e| e.to_string())?
                    .cast()
            } else {
                std::ptr::null_mut()
            };
            Ok(self.res.heaps.insert(Heap {
                memory,
                data,
                shared,
            }))
        }
    }

    pub fn sparse_page(&self) -> Result<u64, String> {
        if !self.caps().sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(1, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let probe = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let alignment = self.raw.get_buffer_memory_requirements(probe).alignment;
            self.raw.destroy_buffer(probe, None);
            Ok(alignment)
        }
    }

    pub fn sparse_tile(&self, info: &TextureInfo) -> Result<[u32; 3], String> {
        let create = image_info(info);
        let formats = unsafe {
            self.instance
                .get_physical_device_sparse_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.samples,
                    create.usage,
                    vk::ImageTiling::OPTIMAL,
                )
        };
        let f = formats
            .first()
            .ok_or(format!("format {} has no sparse layout", info.format))?;
        let g = f.image_granularity;
        Ok([g.width, g.height, g.depth])
    }

    pub fn create_sparse_buffer(&mut self, size: u64) -> Result<u32, String> {
        if !self.caps().sparse {
            return Err("sparse residency is not supported".into());
        }
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self
            .buffer_info(size, self.usage(), &mut wide)
            .flags(vk::BufferCreateFlags::SPARSE_BINDING | vk::BufferCreateFlags::SPARSE_RESIDENCY);
        unsafe {
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let address = self
                .raw
                .get_buffer_device_address(&vk::BufferDeviceAddressInfo::default().buffer(raw));
            Ok(self.insert_buffer(Buffer {
                raw,
                memory: vk::DeviceMemory::null(),
                address,
                data: std::ptr::null_mut(),
                own: true,
            }))
        }
    }

    pub fn create_sparse_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        if !self.caps().sparse_textures {
            return Err("sparse textures are not supported".into());
        }
        let create = image_info(info);
        let create = create.flags(
            create.flags
                | vk::ImageCreateFlags::SPARSE_BINDING
                | vk::ImageCreateFlags::SPARSE_RESIDENCY,
        );
        let formats = unsafe {
            self.instance
                .get_physical_device_sparse_image_format_properties(
                    self.physical,
                    create.format,
                    create.image_type,
                    create.samples,
                    create.usage,
                    vk::ImageTiling::OPTIMAL,
                )
        };
        let tile = formats
            .first()
            .ok_or(format!("format {} has no sparse layout", info.format))?
            .image_granularity;
        let image = unsafe {
            self.raw
                .create_image(&self.share_image(create), None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.register(image, vk::DeviceMemory::null(), true, &whole(info))?;
        self.res.textures.get_mut(slot).tile = tile;
        Ok(slot)
    }

    fn bind_sparse(
        &self,
        queue: usize,
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
        buffers: &[vk::SparseBufferMemoryBindInfo],
        images: &[vk::SparseImageMemoryBindInfo],
    ) -> Result<(), String> {
        let wait: Vec<_> = waits.iter().map(|w| self.res.events.get(w.0).0).collect();
        let wait_values: Vec<_> = waits.iter().map(|w| w.1).collect();
        let signal: Vec<_> = signals.iter().map(|s| self.res.events.get(s.0).0).collect();
        let signal_values: Vec<_> = signals.iter().map(|s| s.1).collect();
        let mut timeline = vk::TimelineSemaphoreSubmitInfo::default()
            .wait_semaphore_values(&wait_values)
            .signal_semaphore_values(&signal_values);
        let info = vk::BindSparseInfo::default()
            .wait_semaphores(&wait)
            .buffer_binds(buffers)
            .image_binds(images)
            .signal_semaphores(&signal)
            .push_next(&mut timeline);
        unsafe {
            self.raw
                .queue_bind_sparse(self.queues[queue].raw, &[info], vk::Fence::null())
                .map_err(|e| e.to_string())
        }
    }

    pub fn map_sparse_buffer(
        &mut self,
        queue: usize,
        buffer: u32,
        heap: u32,
        ranges: &[SparseRange],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let binds: Vec<_> = ranges
            .iter()
            .map(|r| vk::SparseMemoryBind {
                resource_offset: r.page as u64 * page,
                size: r.count as u64 * page,
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseBufferMemoryBindInfo::default()
            .buffer(self.res.buffers.get(buffer).raw)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &infos, &[])
    }

    pub fn map_sparse_texture(
        &mut self,
        queue: usize,
        texture: u32,
        heap: u32,
        regions: &[SparseRegion],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        let page = self.sparse_page()?;
        let memory = if heap == !0 {
            vk::DeviceMemory::null()
        } else {
            self.res.heaps.get(heap).memory
        };
        let t = self.res.textures.get(texture);
        let binds: Vec<_> = regions
            .iter()
            .map(|r| vk::SparseImageMemoryBind {
                subresource: vk::ImageSubresource {
                    aspect_mask: t.aspect,
                    mip_level: r.mip,
                    array_layer: r.layer,
                },
                offset: vk::Offset3D {
                    x: (r.x * t.tile.width) as i32,
                    y: (r.y * t.tile.height) as i32,
                    z: (r.z * t.tile.depth) as i32,
                },
                extent: vk::Extent3D {
                    width: r.width * t.tile.width,
                    height: r.height * t.tile.height,
                    depth: r.depth * t.tile.depth,
                },
                memory,
                memory_offset: r.heap_page as u64 * page,
                flags: vk::SparseMemoryBindFlags::empty(),
            })
            .collect();
        let infos = [vk::SparseImageMemoryBindInfo::default()
            .image(t.image)
            .binds(&binds)];
        self.bind_sparse(queue, waits, signals, &[], &infos)
    }

    pub fn destroy_heap(&mut self, heap: u32) {
        let heap = self.res.heaps.remove(heap);
        unsafe { self.raw.free_memory(heap.memory, None) };
    }

    pub fn buffer_size(&self, size: u64, _memory: u32) -> (u64, u64) {
        let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
        let info = self.buffer_info(size, self.usage(), &mut wide);
        let query = vk::DeviceBufferMemoryRequirements::default().create_info(&info);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_buffer_memory_requirements(&query, &mut needs)
        };
        (
            needs.memory_requirements.size,
            needs.memory_requirements.alignment,
        )
    }

    pub fn texture_size(&self, info: &TextureInfo) -> (u64, u64) {
        let image = self.share_image(image_info(info));
        let query = vk::DeviceImageMemoryRequirements::default().create_info(&image);
        let mut needs = vk::MemoryRequirements2::default();
        unsafe {
            self.raw
                .get_device_image_memory_requirements(&query, &mut needs)
        };
        (
            needs.memory_requirements.size,
            needs.memory_requirements.alignment,
        )
    }

    pub fn create_placed_buffer(
        &mut self,
        heap: u32,
        offset: u64,
        size: u64,
    ) -> Result<u32, String> {
        let h = self.res.heaps.get(heap);
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let mut external = vk::ExternalMemoryBufferCreateInfo::default()
                .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let mut info = self.buffer_info(size, self.usage(), &mut wide);
            if h.shared {
                info = info.push_next(&mut external);
            }
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            if let Err(e) = self.raw.bind_buffer_memory(raw, h.memory, offset) {
                self.raw.destroy_buffer(raw, None);
                return Err(e.to_string());
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            let address = self.raw.get_buffer_device_address(&info);
            let data = if h.data.is_null() {
                h.data
            } else {
                h.data.add(offset as usize)
            };
            Ok(self.insert_buffer(Buffer {
                raw,
                memory: vk::DeviceMemory::null(),
                address,
                data,
                own: true,
            }))
        }
    }

    pub fn create_placed_texture(
        &mut self,
        heap: u32,
        offset: u64,
        info: &TextureInfo,
    ) -> Result<u32, String> {
        let h = self.res.heaps.get(heap);
        let memory = h.memory;
        let mut external = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let mut create = self.share_image(image_info(info));
        if h.shared {
            create = create.push_next(&mut external);
        }
        unsafe {
            let image = self
                .raw
                .create_image(&create, None)
                .map_err(|e| e.to_string())?;
            if let Err(e) = self.raw.bind_image_memory(image, memory, offset) {
                self.raw.destroy_image(image, None);
                return Err(e.to_string());
            }
            self.register(image, vk::DeviceMemory::null(), true, &whole(info))
        }
    }

    fn allocate(
        &self,
        size: u64,
        memory: u32,
        usage: vk::BufferUsageFlags,
    ) -> Result<Buffer, String> {
        unsafe {
            let mut wide = vk::BufferUsageFlags2CreateInfoKHR::default();
            let info = self.buffer_info(size, usage, &mut wide);
            let raw = self
                .raw
                .create_buffer(&info, None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_buffer_memory_requirements(raw);
            let index = match self.memory_type(needs.memory_type_bits, memory) {
                Ok(i) => i,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e);
                }
            };
            let mut flags = vk::MemoryAllocateFlagsInfo::default()
                .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().buffer(raw);
            let info = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut flags)
                .push_next(&mut dedicated);
            let memory = match self.raw.allocate_memory(&info, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_buffer(raw, None);
                    return Err(e.to_string());
                }
            };
            let mut buffer = Buffer {
                raw,
                memory,
                address: 0,
                data: std::ptr::null_mut(),
                own: true,
            };
            if let Err(e) = self.raw.bind_buffer_memory(raw, memory, 0) {
                self.free_buffer(&buffer);
                return Err(e.to_string());
            }
            let visible = self.memory.memory_types[index as usize]
                .property_flags
                .contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            if visible {
                match self
                    .raw
                    .map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                {
                    Ok(data) => buffer.data = data.cast(),
                    Err(e) => {
                        self.free_buffer(&buffer);
                        return Err(e.to_string());
                    }
                }
            }
            let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
            buffer.address = self.raw.get_buffer_device_address(&info);
            Ok(buffer)
        }
    }

    pub fn create_accel(&mut self, info: &AccelInfo) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let (kind, geometry, ranges) = if !info.boxes.is_empty() {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for b in info.boxes {
                let aabbs = vk::AccelerationStructureGeometryAabbsDataKHR::default()
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: self.res.buffers.get(b.buffer).address + b.offset,
                    })
                    .stride(b.stride as u64);
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::AABBS)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { aabbs })
                        .flags(if b.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(b.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        } else if info.triangles.is_empty() {
            let instances = vk::AccelerationStructureGeometryInstancesDataKHR::default().data(
                vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(info.instances).address
                        + info.instance_offset,
                },
            );
            let geometry = vk::AccelerationStructureGeometryKHR::default()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(vk::AccelerationStructureGeometryDataKHR { instances });
            let range = vk::AccelerationStructureBuildRangeInfoKHR::default()
                .primitive_count(info.instance_count);
            (
                vk::AccelerationStructureTypeKHR::TOP_LEVEL,
                vec![geometry],
                vec![range],
            )
        } else {
            let mut geometry = Vec::new();
            let mut ranges = Vec::new();
            for t in info.triangles {
                let address = |b: u32, o: u64| vk::DeviceOrHostAddressConstKHR {
                    device_address: self.res.buffers.get(b).address + o,
                };
                let mut triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
                    .vertex_format(format::vk(t.format))
                    .vertex_data(address(t.vertices, t.vertex_offset))
                    .vertex_stride(t.stride as u64)
                    .max_vertex(t.vertex_count.saturating_sub(1))
                    .index_type(vk::IndexType::NONE_KHR);
                if t.indices != !0 {
                    triangles = triangles
                        .index_type(match t.index_kind {
                            index::U16 => vk::IndexType::UINT16,
                            index::U32 => vk::IndexType::UINT32,
                            k => panic!("unknown index {k}"),
                        })
                        .index_data(address(t.indices, t.index_offset));
                }
                geometry.push(
                    vk::AccelerationStructureGeometryKHR::default()
                        .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                        .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
                        .flags(if t.opaque {
                            vk::GeometryFlagsKHR::OPAQUE
                        } else {
                            vk::GeometryFlagsKHR::empty()
                        }),
                );
                ranges.push(
                    vk::AccelerationStructureBuildRangeInfoKHR::default().primitive_count(t.count),
                );
            }
            (
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                geometry,
                ranges,
            )
        };
        let flags = vk::BuildAccelerationStructureFlagsKHR::from_raw(info.flags);
        let build = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(kind)
            .flags(flags)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometry);
        let counts: Vec<_> = ranges.iter().map(|r| r.primitive_count).collect();
        let mut sizes = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            ray.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build,
                &counts,
                &mut sizes,
            )
        };
        let buffer = self.allocate(
            sizes.acceleration_structure_size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(sizes.acceleration_structure_size)
            .ty(kind);
        let raw = unsafe {
            ray.create_acceleration_structure(&create, None)
                .map_err(|e| e.to_string())?
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry,
            ranges,
            build: sizes.build_scratch_size,
            update: sizes.update_scratch_size,
        }))
    }

    pub fn create_compacted(&mut self, accel: u32, size: u64) -> Result<u32, String> {
        let ray = self
            .ext
            .ray
            .as_ref()
            .ok_or("VK_KHR_acceleration_structure is not enabled")?;
        let source = self.res.accels.get(accel);
        let kind = source.kind;
        let flags = source.flags;
        let buffer = self.allocate(
            size,
            memory::DEVICE,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
        )?;
        let create = vk::AccelerationStructureCreateInfoKHR::default()
            .buffer(buffer.raw)
            .size(size)
            .ty(kind);
        let raw = unsafe {
            ray.create_acceleration_structure(&create, None)
                .map_err(|e| e.to_string())?
        };
        let address = unsafe {
            ray.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(raw),
            )
        };
        Ok(self.res.accels.insert(Accel {
            raw,
            buffer,
            address,
            kind,
            flags,
            geometry: Vec::new(),
            ranges: Vec::new(),
            build: 0,
            update: 0,
        }))
    }

    pub fn destroy_accel(&mut self, accel: u32) {
        let a = self.res.accels.remove(accel);
        unsafe {
            self.ext
                .ray
                .as_ref()
                .unwrap()
                .destroy_acceleration_structure(a.raw, None);
            self.raw.destroy_buffer(a.buffer.raw, None);
            self.raw.free_memory(a.buffer.memory, None);
        }
    }

    pub fn accel_scratch(&self, accel: u32) -> (u64, u64) {
        let a = self.res.accels.get(accel);
        (a.build, a.update)
    }

    pub fn accel_handle(&self, accel: u32) -> u64 {
        self.res.accels.get(accel).address
    }

    pub fn write_instance(
        &self,
        out: &mut [u8],
        accel: u32,
        transform: &[f32; 12],
        id: u32,
        mask: u32,
        flags: u32,
    ) {
        for (i, v) in transform.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out[48..52].copy_from_slice(&(id & 0xff_ffff | mask << 24).to_le_bytes());
        out[52..56].copy_from_slice(&(flags << 24).to_le_bytes());
        out[56..64].copy_from_slice(&self.accel_handle(accel).to_le_bytes());
    }

    pub fn import_buffer(&mut self, raw: vk::Buffer) -> u32 {
        let info = vk::BufferDeviceAddressInfo::default().buffer(raw);
        let address = unsafe { self.raw.get_buffer_device_address(&info) };
        self.insert_buffer(Buffer {
            raw,
            memory: vk::DeviceMemory::null(),
            address,
            data: std::ptr::null_mut(),
            own: false,
        })
    }

    pub fn destroy_buffer(&mut self, buffer: u32) {
        let buffer = self.res.buffers.remove(buffer);
        self.free_buffer(&buffer);
    }

    pub fn raw_buffer(&self, buffer: u32) -> vk::Buffer {
        self.res.buffers.get(buffer).raw
    }

    pub fn raw_texture(&self, texture: u32) -> (vk::Image, vk::ImageView) {
        let t = self.res.textures.get(texture);
        (t.image, t.view)
    }

    pub fn raw_event(&self, event: u32) -> vk::Semaphore {
        self.res.events.get(event).0
    }

    pub fn raw_queue(&self, queue: usize) -> vk::Queue {
        self.queues[queue].raw
    }

    pub fn map(&self, buffer: u32) -> *mut u8 {
        self.res.buffers.get(buffer).data
    }

    pub fn unmap(&self, _buffer: u32) {}

    fn insert_buffer(&mut self, buffer: Buffer) -> u32 {
        let info = vk::DescriptorBufferInfo::default()
            .buffer(buffer.raw)
            .range(vk::WHOLE_SIZE);
        let slot = self.res.buffers.insert(buffer);
        if slot < self.bindless.buffers {
            self.bindless.write_buffer(&self.raw, slot, info);
        }
        slot
    }

    pub fn buffer_handle(&self, buffer: u32) -> Result<u64, String> {
        if buffer >= self.bindless.buffers {
            return Err("the bindless heap has no slot for this buffer".into());
        }
        Ok((self.bindless.images + buffer) as u64)
    }

    pub fn address(&self, buffer: u32) -> u64 {
        self.res.buffers.get(buffer).address
    }

    #[allow(clippy::too_many_arguments)]
    fn graphics(
        &mut self,
        code: &[u8],
        entries: &[(vk::ShaderStageFlags, &str)],
        vertex: Option<&RenderInfo>,
        colors: &[ColorTarget],
        depth: u32,
        stencil: u32,
        samples: u32,
        fill: u32,
        coverage: bool,
        views: u32,
        clamp: bool,
    ) -> Result<u32, String> {
        if clamp && self.ext.features.depth_clamp == 0 {
            return Err("depth clamp is not supported".into());
        }
        if fill == fill::LINE && self.ext.features.fill_mode_non_solid == 0 {
            return Err("line fill is not supported".into());
        }
        unsafe {
            let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
            let module_info = vk::ShaderModuleCreateInfo::default().code(&words);
            let module = self
                .raw
                .create_shader_module(&module_info, None)
                .map_err(|e| e.to_string())?;
            let names: Vec<_> = entries
                .iter()
                .map(|(_, name)| CString::new(*name).unwrap())
                .collect();
            let stages: Vec<_> = entries
                .iter()
                .zip(&names)
                .filter(|((_, name), _)| !name.is_empty())
                .map(|((stage, _), name)| {
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(*stage)
                        .module(module)
                        .name(name)
                })
                .collect();
            let (buffers, attributes) =
                vertex.map_or((&[][..], &[][..]), |v| (v.buffers, v.attributes));
            let bindings: Vec<_> = buffers
                .iter()
                .enumerate()
                .map(|(i, b)| vk::VertexInputBindingDescription {
                    binding: i as u32,
                    stride: b.stride,
                    input_rate: if b.instance {
                        vk::VertexInputRate::INSTANCE
                    } else {
                        vk::VertexInputRate::VERTEX
                    },
                })
                .collect();
            let attributes: Vec<_> = attributes
                .iter()
                .map(|a| vk::VertexInputAttributeDescription {
                    location: a.location,
                    binding: a.buffer,
                    format: format::vk(a.format),
                    offset: a.offset,
                })
                .collect();
            let input = vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&bindings)
                .vertex_attribute_descriptions(&attributes);
            let assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(
                match vertex.map(|v| v.topology) {
                    Some(topology::POINT) => vk::PrimitiveTopology::POINT_LIST,
                    Some(topology::LINE | topology::LINE_STRIP) => vk::PrimitiveTopology::LINE_LIST,
                    None | Some(topology::TRIANGLE | topology::TRIANGLE_STRIP) => {
                        vk::PrimitiveTopology::TRIANGLE_LIST
                    }
                    Some(t) => panic!("unknown topology {t}"),
                },
            );
            let viewport = vk::PipelineViewportStateCreateInfo::default();
            let raster = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(match fill {
                    fill::LINE => vk::PolygonMode::LINE,
                    fill::FILL => vk::PolygonMode::FILL,
                    f => panic!("unknown fill {f}"),
                })
                .depth_clamp_enable(clamp)
                .line_width(1.0);
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::from_raw(samples))
                .alpha_to_coverage_enable(coverage);
            let depth_state = vk::PipelineDepthStencilStateCreateInfo::default();
            let blends: Vec<_> = colors
                .iter()
                .map(|c| vk::PipelineColorBlendAttachmentState {
                    blend_enable: c.blend as u32,
                    src_color_blend_factor: vk::BlendFactor::from_raw(c.src as i32),
                    dst_color_blend_factor: vk::BlendFactor::from_raw(c.dst as i32),
                    color_blend_op: vk::BlendOp::from_raw(c.op as i32),
                    src_alpha_blend_factor: vk::BlendFactor::from_raw(c.src_alpha as i32),
                    dst_alpha_blend_factor: vk::BlendFactor::from_raw(c.dst_alpha as i32),
                    alpha_blend_op: vk::BlendOp::from_raw(c.op_alpha as i32),
                    color_write_mask: vk::ColorComponentFlags::from_raw(c.mask),
                })
                .collect();
            let blend = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blends);
            let mut states = vec![
                vk::DynamicState::VIEWPORT_WITH_COUNT,
                vk::DynamicState::SCISSOR_WITH_COUNT,
                vk::DynamicState::CULL_MODE,
                vk::DynamicState::FRONT_FACE,
                vk::DynamicState::DEPTH_TEST_ENABLE,
                vk::DynamicState::DEPTH_WRITE_ENABLE,
                vk::DynamicState::DEPTH_COMPARE_OP,
                vk::DynamicState::STENCIL_TEST_ENABLE,
                vk::DynamicState::STENCIL_OP,
                vk::DynamicState::STENCIL_COMPARE_MASK,
                vk::DynamicState::STENCIL_WRITE_MASK,
                vk::DynamicState::STENCIL_REFERENCE,
                vk::DynamicState::DEPTH_BIAS,
                vk::DynamicState::DEPTH_BIAS_ENABLE,
                vk::DynamicState::BLEND_CONSTANTS,
                vk::DynamicState::DEPTH_BOUNDS,
                vk::DynamicState::DEPTH_BOUNDS_TEST_ENABLE,
                vk::DynamicState::LINE_WIDTH,
            ];
            if vertex.is_some() {
                states.push(vk::DynamicState::PRIMITIVE_TOPOLOGY);
            }
            if self.ext.rates.is_some() {
                states.push(vk::DynamicState::FRAGMENT_SHADING_RATE_KHR);
            }
            let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&states);
            let formats: Vec<_> = colors.iter().map(|c| format::vk(c.format)).collect();
            let mut rendering = vk::PipelineRenderingCreateInfo::default()
                .view_mask(if views > 1 { (1 << views) - 1 } else { 0 })
                .color_attachment_formats(&formats)
                .depth_attachment_format(format::vk(depth))
                .stencil_attachment_format(format::vk(stencil));
            let mut pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .viewport_state(&viewport)
                .rasterization_state(&raster)
                .multisample_state(&multisample)
                .depth_stencil_state(&depth_state)
                .color_blend_state(&blend)
                .dynamic_state(&dynamic)
                .layout(self.layout)
                .push_next(&mut rendering);
            if vertex.is_some() {
                pipeline_info = pipeline_info
                    .vertex_input_state(&input)
                    .input_assembly_state(&assembly);
            }
            let result = self
                .raw
                .create_graphics_pipelines(self.cache, &[pipeline_info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.res.renders.insert(pipeline))
        }
    }

    pub fn create_render_pipeline(&mut self, info: &RenderInfo) -> Result<u32, String> {
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::VERTEX, info.vertex),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            Some(info),
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.views,
            info.clamp,
        )
    }

    pub fn create_mesh_pipeline(&mut self, info: &MeshInfo) -> Result<u32, String> {
        if self.ext.mesh.is_none() {
            return Err("VK_EXT_mesh_shader is not enabled".into());
        }
        self.graphics(
            info.code,
            &[
                (vk::ShaderStageFlags::TASK_EXT, info.task),
                (vk::ShaderStageFlags::MESH_EXT, info.mesh),
                (vk::ShaderStageFlags::FRAGMENT, info.fragment),
            ],
            None,
            info.colors,
            info.depth,
            info.stencil,
            info.samples,
            info.fill,
            info.coverage,
            info.views,
            info.clamp,
        )
    }

    pub fn destroy_render_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.res.renders.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_depth_state(&mut self, info: &DepthInfo) -> Result<u32, String> {
        Ok(self.res.depths.insert(*info))
    }

    pub fn destroy_depth_state(&mut self, state: u32) {
        self.res.depths.remove(state);
    }

    pub fn create_texture(&mut self, info: &TextureInfo) -> Result<u32, String> {
        unsafe {
            let image = self
                .raw
                .create_image(&self.share_image(image_info(info)), None)
                .map_err(|e| e.to_string())?;
            let needs = self.raw.get_image_memory_requirements(image);
            let index = match self.memory_type(needs.memory_type_bits, info.memory) {
                Ok(i) => i,
                Err(e) => {
                    self.raw.destroy_image(image, None);
                    return Err(e);
                }
            };
            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
            let alloc = vk::MemoryAllocateInfo::default()
                .allocation_size(needs.size)
                .memory_type_index(index)
                .push_next(&mut dedicated);
            let memory = match self.raw.allocate_memory(&alloc, None) {
                Ok(m) => m,
                Err(e) => {
                    self.raw.destroy_image(image, None);
                    return Err(e.to_string());
                }
            };
            if let Err(e) = self.raw.bind_image_memory(image, memory, 0) {
                self.raw.destroy_image(image, None);
                self.raw.free_memory(memory, None);
                return Err(e.to_string());
            }
            self.register(image, memory, true, &whole(info))
        }
    }

    pub fn import_texture(&mut self, image: vk::Image, info: &TextureInfo) -> Result<u32, String> {
        self.register(image, vk::DeviceMemory::null(), false, &whole(info))
    }

    pub fn create_view(&mut self, texture: u32, info: &ViewInfo) -> Result<u32, String> {
        let image = self.res.textures.get(texture).image;
        self.register(image, vk::DeviceMemory::null(), false, info)
    }

    fn register(
        &mut self,
        image: vk::Image,
        memory: vk::DeviceMemory,
        own: bool,
        info: &ViewInfo,
    ) -> Result<u32, String> {
        let view = match info.dimension {
            dimension::D1 => vk::ImageViewType::TYPE_1D,
            dimension::D1_ARRAY => vk::ImageViewType::TYPE_1D_ARRAY,
            dimension::D2 => vk::ImageViewType::TYPE_2D,
            dimension::D2_ARRAY => vk::ImageViewType::TYPE_2D_ARRAY,
            dimension::D3 => vk::ImageViewType::TYPE_3D,
            dimension::CUBE => vk::ImageViewType::CUBE,
            dimension::CUBE_ARRAY => vk::ImageViewType::CUBE_ARRAY,
            d => panic!("unknown dimension {d}"),
        };
        let native = format::vk(info.format);
        unsafe {
            let aspect = aspect(info.format);
            let range = vk::ImageSubresourceRange {
                aspect_mask: if aspect.contains(vk::ImageAspectFlags::DEPTH) {
                    vk::ImageAspectFlags::DEPTH
                } else {
                    aspect
                },
                base_mip_level: info.base_mip,
                level_count: info.mips,
                base_array_layer: info.base_layer,
                layer_count: info.layers,
            };
            let view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(view)
                .format(native)
                .subresource_range(range);
            let view = match self.raw.create_image_view(&view_info, None) {
                Ok(v) => v,
                Err(e) => {
                    if own {
                        self.raw.destroy_image(image, None);
                    }
                    if memory != vk::DeviceMemory::null() {
                        self.raw.free_memory(memory, None);
                    }
                    return Err(e.to_string());
                }
            };
            let slot = self.res.textures.insert(Texture {
                image,
                memory,
                view,
                aspect,
                own,
                tile: vk::Extent3D::default(),
            });
            let bound = info.usage & (usage::SAMPLED | usage::STORAGE) != 0;
            if bound && slot * 2 + 1 >= self.bindless.images {
                let t = self.res.textures.remove(slot);
                self.free_texture(&t);
                return Err("the bindless heap has no texture slot left".into());
            }
            if info.usage & usage::SAMPLED != 0 {
                let image = vk::DescriptorImageInfo::default()
                    .image_view(view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                self.bindless.write(
                    &self.raw,
                    2,
                    slot * 2,
                    vk::DescriptorType::SAMPLED_IMAGE,
                    image,
                );
            }
            if info.usage & usage::STORAGE != 0 {
                let image = vk::DescriptorImageInfo::default()
                    .image_view(view)
                    .image_layout(vk::ImageLayout::GENERAL);
                self.bindless.write(
                    &self.raw,
                    2,
                    slot * 2 + 1,
                    vk::DescriptorType::STORAGE_IMAGE,
                    image,
                );
            }
            Ok(slot)
        }
    }

    pub fn destroy_texture(&mut self, texture: u32) {
        let texture = self.res.textures.remove(texture);
        self.free_texture(&texture);
    }

    fn free_texture(&self, texture: &Texture) {
        unsafe {
            self.raw.destroy_image_view(texture.view, None);
            if texture.own {
                self.raw.destroy_image(texture.image, None);
            }
            if texture.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(texture.memory, None);
            }
        }
    }

    pub fn texture_handle(&self, texture: u32) -> u64 {
        texture as u64 * 2
    }

    pub fn image_handle(&self, texture: u32) -> u64 {
        texture as u64 * 2 + 1
    }

    pub fn create_sampler(&mut self, info: &SamplerInfo) -> Result<u32, String> {
        if info.anisotropy > 1 && self.ext.features.sampler_anisotropy == 0 {
            return Err("anisotropic filtering is not supported".into());
        }
        let filter = |f| match f {
            filter::NEAREST => vk::Filter::NEAREST,
            filter::LINEAR => vk::Filter::LINEAR,
            f => panic!("unknown filter {f}"),
        };
        let address = |a| match a {
            address::REPEAT => vk::SamplerAddressMode::REPEAT,
            address::MIRROR => vk::SamplerAddressMode::MIRRORED_REPEAT,
            address::CLAMP => vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address::BORDER => vk::SamplerAddressMode::CLAMP_TO_BORDER,
            address::MIRROR_CLAMP => vk::SamplerAddressMode::MIRROR_CLAMP_TO_EDGE,
            a => panic!("unknown address {a}"),
        };
        let sampler_info = vk::SamplerCreateInfo::default()
            .min_filter(filter(info.min))
            .mag_filter(filter(info.mag))
            .mipmap_mode(match info.mip {
                filter::NEAREST => vk::SamplerMipmapMode::NEAREST,
                filter::LINEAR => vk::SamplerMipmapMode::LINEAR,
                f => panic!("unknown filter {f}"),
            })
            .address_mode_u(address(info.u))
            .address_mode_v(address(info.v))
            .address_mode_w(address(info.w))
            .anisotropy_enable(info.anisotropy > 1)
            .max_anisotropy(info.anisotropy as f32)
            .compare_enable(info.compare != 0)
            .compare_op(vk::CompareOp::from_raw(
                info.compare.saturating_sub(1) as i32
            ))
            .min_lod(info.min_lod)
            .max_lod(info.max_lod)
            .border_color(match info.border {
                border::OPAQUE_BLACK => vk::BorderColor::FLOAT_OPAQUE_BLACK,
                border::OPAQUE_WHITE => vk::BorderColor::FLOAT_OPAQUE_WHITE,
                border::TRANSPARENT_BLACK => vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                b => panic!("unknown border {b}"),
            });
        let sampler = unsafe {
            self.raw
                .create_sampler(&sampler_info, None)
                .map_err(|e| e.to_string())?
        };
        let slot = self.res.samplers.insert(sampler);
        if slot >= self.bindless.samplers {
            self.res.samplers.remove(slot);
            unsafe { self.raw.destroy_sampler(sampler, None) };
            return Err("the bindless heap has no sampler slot left".into());
        }
        let image = vk::DescriptorImageInfo::default().sampler(sampler);
        self.bindless
            .write(&self.raw, 0, slot, vk::DescriptorType::SAMPLER, image);
        Ok(slot)
    }

    pub fn destroy_sampler(&mut self, sampler: u32) {
        let sampler = self.res.samplers.remove(sampler);
        unsafe { self.raw.destroy_sampler(sampler, None) };
    }

    pub fn sampler_handle(&self, sampler: u32) -> u64 {
        sampler as u64
    }

    pub fn record_cache(&mut self) -> Result<(), String> {
        if self.cache == vk::PipelineCache::null() {
            self.cache = self.new_cache(&[])?;
        }
        Ok(())
    }

    pub fn save_cache(&self, path: &str) -> Result<(), String> {
        let data = unsafe {
            self.raw
                .get_pipeline_cache_data(self.cache)
                .map_err(|e| e.to_string())?
        };
        std::fs::write(path, data).map_err(|e| e.to_string())
    }

    pub fn load_cache(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let cache = self.new_cache(&data)?;
        unsafe { self.raw.destroy_pipeline_cache(self.cache, None) };
        self.cache = cache;
        Ok(())
    }

    fn new_cache(&self, data: &[u8]) -> Result<vk::PipelineCache, String> {
        let info = vk::PipelineCacheCreateInfo::default().initial_data(data);
        unsafe {
            self.raw
                .create_pipeline_cache(&info, None)
                .map_err(|e| e.to_string())
        }
    }

    pub fn create_pipeline(&mut self, info: &ComputeInfo) -> Result<u32, String> {
        if !info.functions.is_empty() {
            return Err("linked functions are Metal only".into());
        }
        let (code, entry) = (info.code, info.entry);
        unsafe {
            let words = ash::util::read_spv(&mut Cursor::new(code)).map_err(|e| e.to_string())?;
            let info = vk::ShaderModuleCreateInfo::default().code(&words);
            let module = self
                .raw
                .create_shader_module(&info, None)
                .map_err(|e| e.to_string())?;
            let name = CString::new(entry).unwrap();
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(&name);
            let info = vk::ComputePipelineCreateInfo::default()
                .stage(stage)
                .layout(self.layout);
            let result = self.raw.create_compute_pipelines(self.cache, &[info], None);
            self.raw.destroy_shader_module(module, None);
            let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
            Ok(self.res.pipelines.insert(pipeline))
        }
    }

    pub fn destroy_pipeline(&mut self, pipeline: u32) {
        let pipeline = self.res.pipelines.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(pipeline, None) };
    }

    pub fn create_surface(
        &mut self,
        window: &(impl HasWindowHandle + HasDisplayHandle),
        info: &SurfaceInfo,
    ) -> Result<u32, String> {
        let display = window.display_handle().map_err(|e| e.to_string())?;
        let handle = window.window_handle().map_err(|e| e.to_string())?;
        let raw = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                display.as_raw(),
                handle.as_raw(),
                None,
            )
            .map_err(|e| e.to_string())?
        };
        self.surface(raw, true, info)
    }

    pub fn create_surface_raw(&mut self, surface: u64, info: &SurfaceInfo) -> Result<u32, String> {
        self.surface(vk::Handle::from_raw(surface), false, info)
    }

    fn surface(
        &mut self,
        raw: vk::SurfaceKHR,
        own: bool,
        info: &SurfaceInfo,
    ) -> Result<u32, String> {
        let mut s = Surface {
            raw,
            own,
            swapchain: vk::SwapchainKHR::null(),
            format: info.format,
            vsync: info.vsync,
            count: info.images,
            usage: info.usage,
            size: [0; 2],
            suboptimal: false,
            images: Vec::new(),
            acquires: Vec::new(),
            presents: Vec::new(),
            next: 0,
            index: 0,
        };
        self.swapchain(&mut s, info.width, info.height)?;
        Ok(self.res.surfaces.insert(s))
    }

    fn swapchain(&mut self, s: &mut Surface, width: u32, height: u32) -> Result<(), String> {
        let (surfaces, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        unsafe {
            let caps = surfaces
                .get_physical_device_surface_capabilities(self.physical, s.raw)
                .map_err(|e| e.to_string())?;
            let modes = surfaces
                .get_physical_device_surface_present_modes(self.physical, s.raw)
                .map_err(|e| e.to_string())?;
            let mode = if s.vsync {
                vk::PresentModeKHR::FIFO
            } else {
                [vk::PresentModeKHR::MAILBOX, vk::PresentModeKHR::IMMEDIATE]
                    .into_iter()
                    .find(|m| modes.contains(m))
                    .ok_or("the surface has no present mode without vsync")?
            };
            let max = if caps.max_image_count == 0 {
                u32::MAX
            } else {
                caps.max_image_count
            };
            if s.count < caps.min_image_count || s.count > max {
                return Err(format!(
                    "{} images is outside the surface range {}..={max}",
                    s.count, caps.min_image_count
                ));
            }
            let extent = if caps.current_extent.width == u32::MAX {
                vk::Extent2D { width, height }
            } else {
                caps.current_extent
            };
            s.size = [extent.width, extent.height];
            let usage = image_usage(s.usage);
            if !caps.supported_usage_flags.contains(usage) {
                return Err(format!("the surface does not support usage {}", s.usage));
            }
            let native = format::vk(s.format);
            let old = s.swapchain;
            let info = vk::SwapchainCreateInfoKHR::default()
                .surface(s.raw)
                .min_image_count(s.count)
                .image_format(native)
                .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                .image_extent(extent)
                .image_array_layers(1)
                .image_usage(usage)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(caps.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(mode)
                .clipped(true)
                .old_swapchain(old);
            s.swapchain = swapchains
                .create_swapchain(&info, None)
                .map_err(|e| e.to_string())?;
            for slot in s.images.drain(..) {
                let t = self.res.textures.remove(slot);
                self.raw.destroy_image_view(t.view, None);
            }
            if old != vk::SwapchainKHR::null() {
                swapchains.destroy_swapchain(old, None);
            }
            for image in swapchains
                .get_swapchain_images(s.swapchain)
                .map_err(|e| e.to_string())?
            {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(native)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                let view = self.raw.create_image_view(&view_info, None).unwrap();
                s.images.push(self.res.textures.insert(Texture {
                    image,
                    memory: vk::DeviceMemory::null(),
                    view,
                    aspect: vk::ImageAspectFlags::COLOR,
                    own: false,
                    tile: vk::Extent3D::default(),
                }));
            }
            let binary = vk::SemaphoreCreateInfo::default();
            while s.presents.len() < s.images.len() {
                s.presents
                    .push(self.raw.create_semaphore(&binary, None).unwrap());
            }
            while s.acquires.len() < s.images.len() + 1 {
                s.acquires
                    .push((self.raw.create_semaphore(&binary, None).unwrap(), 0));
            }
        }
        Ok(())
    }

    pub fn resize_surface(&mut self, surface: u32, width: u32, height: u32) -> Result<(), String> {
        let mut s = self.res.surfaces.remove(surface);
        let result = self.swapchain(&mut s, width, height);
        self.res.surfaces.insert(s);
        result
    }

    pub fn destroy_surface(&mut self, surface: u32) {
        let s = self.res.surfaces.remove(surface);
        let (surfaces, swapchains) = self.ext.surfaces.as_ref().unwrap();
        unsafe {
            for slot in s.images {
                let t = self.res.textures.remove(slot);
                self.raw.destroy_image_view(t.view, None);
            }
            for (semaphore, _) in s.acquires {
                self.raw.destroy_semaphore(semaphore, None);
            }
            for semaphore in s.presents {
                self.raw.destroy_semaphore(semaphore, None);
            }
            swapchains.destroy_swapchain(s.swapchain, None);
            if s.own {
                surfaces.destroy_surface(s.raw, None);
            }
        }
    }

    pub fn acquire(&mut self, queue: usize, surface: u32) -> Result<u32, String> {
        let (_, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        let s = self.res.surfaces.get_mut(surface);
        let q = &mut self.queues[queue];
        let (semaphore, used) = s.acquires[s.next];
        q.done(&self.raw, used)?;
        let (index, suboptimal) = unsafe {
            swapchains
                .acquire_next_image(s.swapchain, u64::MAX, semaphore, vk::Fence::null())
                .map_err(|e| e.to_string())?
        };
        s.suboptimal = suboptimal;
        s.acquires[s.next].1 = q.sync(&self.raw, semaphore, vk::Semaphore::null())?;
        s.next = (s.next + 1) % s.acquires.len();
        s.index = index;
        Ok(s.images[index as usize])
    }

    pub fn surface_size(&self, surface: u32) -> [u32; 2] {
        self.res.surfaces.get(surface).size
    }

    pub fn present(&mut self, queue: usize, surface: u32) -> Result<bool, String> {
        let (_, swapchains) = self
            .ext
            .surfaces
            .as_ref()
            .ok_or("VK_KHR_swapchain is not enabled")?;
        let s = self.res.surfaces.get(surface);
        let q = &mut self.queues[queue];
        let semaphore = s.presents[s.index as usize];
        q.sync(&self.raw, vk::Semaphore::null(), semaphore)?;
        let waits = [semaphore];
        let swapchain = [s.swapchain];
        let indices = [s.index];
        let info = vk::PresentInfoKHR::default()
            .wait_semaphores(&waits)
            .swapchains(&swapchain)
            .image_indices(&indices);
        let suboptimal = unsafe {
            swapchains
                .queue_present(q.raw, &info)
                .map_err(|e| e.to_string())?
        };
        Ok(suboptimal || s.suboptimal)
    }

    pub fn create_ray_pipeline(&mut self, info: &RayInfo) -> Result<u32, String> {
        let rays = self
            .ext
            .rays
            .as_ref()
            .ok_or("VK_KHR_ray_tracing_pipeline is not enabled")?;
        let words = ash::util::read_spv(&mut Cursor::new(info.code)).map_err(|e| e.to_string())?;
        let module_info = vk::ShaderModuleCreateInfo::default().code(&words);
        let module = unsafe {
            self.raw
                .create_shader_module(&module_info, None)
                .map_err(|e| e.to_string())?
        };
        let mut names = Vec::new();
        let mut stages = Vec::new();
        let mut stage = |name: &str, kind: vk::ShaderStageFlags| {
            if name.is_empty() {
                return vk::SHADER_UNUSED_KHR;
            }
            names.push(CString::new(name).unwrap());
            stages.push(kind);
            stages.len() as u32 - 1
        };
        let general = |shader| {
            vk::RayTracingShaderGroupCreateInfoKHR::default()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(shader)
                .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                .any_hit_shader(vk::SHADER_UNUSED_KHR)
                .intersection_shader(vk::SHADER_UNUSED_KHR)
        };
        let mut groups = vec![general(stage(
            info.raygen,
            vk::ShaderStageFlags::RAYGEN_KHR,
        ))];
        for m in info.misses {
            groups.push(general(stage(m, vk::ShaderStageFlags::MISS_KHR)));
        }
        for i in 0..info
            .closest
            .len()
            .max(info.any.len())
            .max(info.intersection.len())
        {
            let closest = info.closest.get(i).copied().unwrap_or("");
            let any = info.any.get(i).copied().unwrap_or("");
            let intersection = info.intersection.get(i).copied().unwrap_or("");
            let closest = stage(closest, vk::ShaderStageFlags::CLOSEST_HIT_KHR);
            let any = stage(any, vk::ShaderStageFlags::ANY_HIT_KHR);
            let intersection = stage(intersection, vk::ShaderStageFlags::INTERSECTION_KHR);
            groups.push(
                vk::RayTracingShaderGroupCreateInfoKHR::default()
                    .ty(if intersection == vk::SHADER_UNUSED_KHR {
                        vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP
                    } else {
                        vk::RayTracingShaderGroupTypeKHR::PROCEDURAL_HIT_GROUP
                    })
                    .general_shader(vk::SHADER_UNUSED_KHR)
                    .closest_hit_shader(closest)
                    .any_hit_shader(any)
                    .intersection_shader(intersection),
            );
        }
        for c in info.callables {
            groups.push(general(stage(c, vk::ShaderStageFlags::CALLABLE_KHR)));
        }
        let infos: Vec<_> = stages
            .iter()
            .zip(&names)
            .map(|(kind, name)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(*kind)
                    .module(module)
                    .name(name)
            })
            .collect();
        let create = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&infos)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(info.depth)
            .layout(self.layout);
        let result = unsafe {
            rays.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(),
                self.cache,
                &[create],
                None,
            )
        };
        unsafe { self.raw.destroy_shader_module(module, None) };
        let pipeline = result.map_err(|(_, e)| e.to_string())?[0];
        Ok(self.res.rays.insert((pipeline, groups.len() as u32)))
    }

    pub fn ray_handles(&self, pipeline: u32) -> Result<Vec<u8>, String> {
        let (raw, count) = *self.res.rays.get(pipeline);
        let size = self.caps().handle_size as usize;
        unsafe {
            self.ext
                .rays
                .as_ref()
                .unwrap()
                .get_ray_tracing_shader_group_handles(raw, 0, count, size * count as usize)
                .map_err(|e| e.to_string())
        }
    }

    pub fn destroy_ray_pipeline(&mut self, pipeline: u32) {
        let (raw, _) = self.res.rays.remove(pipeline);
        unsafe { self.raw.destroy_pipeline(raw, None) };
    }

    pub fn create_commands_layout(&mut self, info: &LayoutInfo) -> Result<u32, String> {
        let generated = self
            .ext
            .generated
            .as_ref()
            .ok_or("VK_EXT_device_generated_commands is not enabled")?;
        let mesh = info
            .tokens
            .iter()
            .any(|t| matches!(t.kind, token::DRAW_MESH | token::DRAW_MESH_COUNT));
        let stages = if info.compute {
            vk::ShaderStageFlags::COMPUTE
        } else if mesh {
            vk::ShaderStageFlags::TASK_EXT
                | vk::ShaderStageFlags::MESH_EXT
                | vk::ShaderStageFlags::FRAGMENT
        } else {
            vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT
        };
        let data: Vec<[u32; 3]> = info
            .tokens
            .iter()
            .map(|t| match t.kind {
                token::PUSH | token::SEQUENCE_INDEX => [
                    vk::ShaderStageFlags::ALL.as_raw(),
                    t.push_offset,
                    t.push_size,
                ],
                _ => [t.binding, 0, 0],
            })
            .collect();
        let tokens: Vec<_> = info
            .tokens
            .iter()
            .zip(&data)
            .map(|(t, d)| ffi::Token {
                s_type: ffi::TOKEN,
                next: std::ptr::null(),
                kind: t.kind as i32,
                data: d.as_ptr().cast(),
                offset: t.offset,
            })
            .collect();
        let create = ffi::LayoutInfo {
            s_type: ffi::LAYOUT,
            next: std::ptr::null(),
            flags: 0,
            stages,
            stride: info.stride,
            layout: self.layout,
            count: tokens.len() as u32,
            tokens: tokens.as_ptr(),
        };
        let mut handle = 0;
        let result = unsafe {
            (generated.create)(self.raw.handle(), &create, std::ptr::null(), &mut handle)
        };
        if result != vk::Result::SUCCESS {
            return Err(result.to_string());
        }
        Ok(self.res.layouts.insert((handle, stages)))
    }

    pub fn destroy_commands_layout(&mut self, layout: u32) {
        let (handle, _) = self.res.layouts.remove(layout);
        let generated = self.ext.generated.as_ref().unwrap();
        unsafe { (generated.destroy)(self.raw.handle(), handle, std::ptr::null()) };
    }

    pub fn generated_size(&self, layout: u32, pipeline: u32, sequences: u32, draws: u32) -> u64 {
        let (handle, stages) = *self.res.layouts.get(layout);
        let pipeline = if stages == vk::ShaderStageFlags::COMPUTE {
            *self.res.pipelines.get(pipeline)
        } else {
            *self.res.renders.get(pipeline)
        };
        let mut bound = ffi::PipelineInfo {
            s_type: ffi::PIPELINE,
            next: std::ptr::null_mut(),
            pipeline,
        };
        let info = ffi::RequirementsInfo {
            s_type: ffi::REQUIREMENTS,
            next: (&raw mut bound).cast(),
            set: 0,
            layout: handle,
            sequences,
            draws,
        };
        let mut needs = vk::MemoryRequirements2::default();
        let generated = self.ext.generated.as_ref().unwrap();
        unsafe { (generated.requirements)(self.raw.handle(), &info, &mut needs) };
        needs.memory_requirements.size
    }

    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let caps = self.caps();
        let none = vk::QueryControlFlags::empty();
        let (kind, flags) = match kind {
            query::TIMESTAMP if !caps.timestamps => {
                return Err("timestamps are not supported".into());
            }
            query::TIMESTAMP => (vk::QueryType::TIMESTAMP, none),
            query::STATISTICS if !caps.statistics => {
                return Err("pipeline statistics are not supported".into());
            }
            query::STATISTICS => (vk::QueryType::PIPELINE_STATISTICS, none),
            query::COMPACTED if !caps.ray => {
                return Err("VK_KHR_acceleration_structure is not enabled".into());
            }
            query::COMPACTED => (
                vk::QueryType::ACCELERATION_STRUCTURE_COMPACTED_SIZE_KHR,
                none,
            ),
            query::OCCLUSION => (vk::QueryType::OCCLUSION, none),
            query::PRECISE if !caps.precise => {
                return Err("precise occlusion queries are not supported".into());
            }
            query::PRECISE => (vk::QueryType::OCCLUSION, vk::QueryControlFlags::PRECISE),
            k => panic!("unknown query kind {k}"),
        };
        let info = vk::QueryPoolCreateInfo::default()
            .query_type(kind)
            .query_count(count)
            .pipeline_statistics(if kind == vk::QueryType::PIPELINE_STATISTICS {
                vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_VERTICES
                    | vk::QueryPipelineStatisticFlags::INPUT_ASSEMBLY_PRIMITIVES
                    | vk::QueryPipelineStatisticFlags::VERTEX_SHADER_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::CLIPPING_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::CLIPPING_PRIMITIVES
                    | vk::QueryPipelineStatisticFlags::FRAGMENT_SHADER_INVOCATIONS
                    | vk::QueryPipelineStatisticFlags::COMPUTE_SHADER_INVOCATIONS
            } else {
                vk::QueryPipelineStatisticFlags::empty()
            });
        let pool = unsafe {
            self.raw
                .create_query_pool(&info, None)
                .map_err(|e| e.to_string())?
        };
        let stride = if kind == vk::QueryType::PIPELINE_STATISTICS {
            56
        } else {
            8
        };
        Ok(self.res.queries.insert((pool, flags, stride)))
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        let (pool, _, _) = self.res.queries.remove(heap);
        unsafe { self.raw.destroy_query_pool(pool, None) };
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        let event = timeline(&self.raw).map_err(|e| e.to_string())?;
        Ok(self.res.events.insert((event, true)))
    }

    pub fn import_event(&mut self, raw: vk::Semaphore) -> u32 {
        self.res.events.insert((raw, false))
    }

    pub fn destroy_event(&mut self, event: u32) {
        let (event, owned) = self.res.events.remove(event);
        if owned {
            unsafe { self.raw.destroy_semaphore(event, None) };
        }
    }

    pub fn event_value(&self, event: u32) -> Result<u64, String> {
        unsafe {
            self.raw
                .get_semaphore_counter_value(self.res.events.get(event).0)
                .map_err(|e| e.to_string())
        }
    }

    pub fn wait(&self, event: u32, value: u64, timeout: u64) -> Result<bool, String> {
        let semaphores = [self.res.events.get(event).0];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe {
            match self
                .raw
                .wait_semaphores(&info, timeout.saturating_mul(1_000_000))
            {
                Ok(()) => Ok(true),
                Err(vk::Result::TIMEOUT) => Ok(false),
                Err(e) => Err(e.to_string()),
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
        self.queues[queue].submit(
            &self.raw,
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
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
        self.queues[queue].submit(
            &self.raw,
            &self.ext,
            self.layout,
            self.bindless.set,
            &self.res,
            (commands, &[]),
            waits,
            signals,
        )
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            if self.owned {
                self.raw.device_wait_idle().ok();
            }
            for queue in &mut self.queues {
                queue.destroy(&self.raw);
            }
            let buffers: Vec<_> = self.res.buffers.drain().collect();
            for buffer in buffers {
                self.free_buffer(&buffer);
            }
            let textures: Vec<_> = self.res.textures.drain().collect();
            for texture in textures {
                self.free_texture(&texture);
            }
            let surfaces: Vec<_> = self.res.surfaces.drain().collect();
            for s in surfaces {
                let (surfaces, swapchains) = self.ext.surfaces.as_ref().unwrap();
                for (semaphore, _) in s.acquires {
                    self.raw.destroy_semaphore(semaphore, None);
                }
                for semaphore in s.presents {
                    self.raw.destroy_semaphore(semaphore, None);
                }
                swapchains.destroy_swapchain(s.swapchain, None);
                if s.own {
                    surfaces.destroy_surface(s.raw, None);
                }
            }
            for sampler in self.res.samplers.drain() {
                self.raw.destroy_sampler(sampler, None);
            }
            let accels: Vec<_> = self.res.accels.drain().collect();
            for a in accels {
                self.ext
                    .ray
                    .as_ref()
                    .unwrap()
                    .destroy_acceleration_structure(a.raw, None);
                self.raw.destroy_buffer(a.buffer.raw, None);
                self.raw.free_memory(a.buffer.memory, None);
            }
            let layouts: Vec<_> = self.res.layouts.drain().collect();
            for (handle, _) in layouts {
                let generated = self.ext.generated.as_ref().unwrap();
                (generated.destroy)(self.raw.handle(), handle, std::ptr::null());
            }
            for (pool, _, _) in self.res.queries.drain() {
                self.raw.destroy_query_pool(pool, None);
            }
            for (pipeline, _) in self.res.rays.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in self.res.renders.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for pipeline in self.res.pipelines.drain() {
                self.raw.destroy_pipeline(pipeline, None);
            }
            for (event, owned) in self.res.events.drain() {
                if owned {
                    self.raw.destroy_semaphore(event, None);
                }
            }
            for heap in self.res.heaps.drain() {
                self.raw.free_memory(heap.memory, None);
            }
            self.raw.destroy_pipeline_cache(self.cache, None);
            self.raw.destroy_pipeline_layout(self.layout, None);
            self.bindless.destroy(&self.raw);
            self.raw.destroy_descriptor_set_layout(self.slots, None);
            if self.owned {
                self.raw.destroy_device(None);
                self.instance.destroy_instance(None);
            }
        }
    }
}
