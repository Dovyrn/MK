use mk::ash::ext::{
    conditional_rendering, debug_utils, external_memory_host, mesh_shader, mutable_descriptor_type,
};
use mk::ash::khr::{
    acceleration_structure, deferred_host_operations, external_memory_fd, fragment_shading_rate,
    maintenance5, portability_enumeration, portability_subset, push_descriptor, ray_query,
    ray_tracing_pipeline,
};
use mk::ash::{self, vk};
use std::ffi::{CStr, c_void};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

const GENERATED: vk::StructureType = vk::StructureType::from_raw(1000572000);

#[repr(C)]
struct Generated {
    s_type: vk::StructureType,
    next: *mut c_void,
    generated: vk::Bool32,
    dynamic: vk::Bool32,
}

pub struct Host {
    device: ManuallyDrop<mk::Device>,
    raw: ash::Device,
    instance: ash::Instance,
    _entry: ash::Entry,
}

impl Deref for Host {
    type Target = mk::Device;

    fn deref(&self) -> &mk::Device {
        &self.device
    }
}

impl DerefMut for Host {
    fn deref_mut(&mut self) -> &mut mk::Device {
        &mut self.device
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        unsafe {
            self.raw.device_wait_idle().ok();
            ManuallyDrop::drop(&mut self.device);
            self.raw.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

fn has(list: &[vk::ExtensionProperties], name: &CStr) -> bool {
    list.iter().any(|e| e.extension_name_as_c_str() == Ok(name))
}

pub fn device(queues: usize) -> Host {
    try_device(queues).expect("not enough queues")
}

pub fn try_device(queues: usize) -> Option<Host> {
    unsafe {
        let entry = ash::Entry::load().unwrap();
        let available = entry.enumerate_instance_extension_properties(None).unwrap();
        let mut extensions = Vec::new();
        let mut flags = vk::InstanceCreateFlags::empty();
        if has(&available, debug_utils::NAME) {
            extensions.push(debug_utils::NAME.as_ptr());
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
        let instance = entry.create_instance(&info, None).unwrap();
        let physicals = instance.enumerate_physical_devices().unwrap();
        let physical = *physicals
            .iter()
            .find(|&&p| {
                instance.get_physical_device_properties(p).device_type
                    == vk::PhysicalDeviceType::DISCRETE_GPU
            })
            .or(physicals.first())
            .expect("no Vulkan device");
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
            instance.destroy_instance(None);
            return None;
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
            .unwrap();
        let mut extensions = vec![mutable_descriptor_type::NAME.as_ptr()];
        if has(&available, portability_subset::NAME) {
            extensions.push(portability_subset::NAME.as_ptr());
        }
        if has(&available, push_descriptor::NAME) {
            extensions.push(push_descriptor::NAME.as_ptr());
        }
        let mut supported_generated = Generated {
            s_type: GENERATED,
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
        let mut enabled_generated = Generated {
            s_type: GENERATED,
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
            .primitive_fragment_shading_rate(supported_rate.primitive_fragment_shading_rate != 0)
            .attachment_fragment_shading_rate(supported_rate.attachment_fragment_shading_rate != 0);
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
        let mut rt =
            vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true);
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
        let raw = instance.create_device(physical, &info, None).unwrap();
        let mut enabled = vk::PhysicalDeviceFeatures2::default().features(features);
        enabled.p_next = info.p_next as *mut _;
        let device =
            mk::Device::adopt(instance.clone(), physical, raw.clone(), &slots, &enabled).unwrap();
        Some(Host {
            device: ManuallyDrop::new(device),
            raw,
            instance,
            _entry: entry,
        })
    }
}
