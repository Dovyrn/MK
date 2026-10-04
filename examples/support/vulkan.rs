use mk::ash::ext::{debug_utils, host_image_copy, mesh_shader, shader_image_atomic_int64};
use mk::ash::khr::{
    acceleration_structure, cooperative_matrix, deferred_host_operations, fragment_shading_rate,
    portability_enumeration, portability_subset, push_descriptor, ray_query,
};
use mk::ash::{self, vk};
use std::ffi::{CStr, c_void};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

pub struct Host {
    device: ManuallyDrop<mk::Device>,
    raw: ash::Device,
    instance: ash::Instance,
    pub entry: ash::Entry,
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
        if instance
            .get_physical_device_properties(physical)
            .api_version
            < vk::API_VERSION_1_3
        {
            instance.destroy_instance(None);
            return None;
        }
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
        let mut supported_v11 = vk::PhysicalDeviceVulkan11Features::default();
        let mut supported_v13 = vk::PhysicalDeviceVulkan13Features::default();
        let mut query = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut supported_v11)
            .push_next(&mut supported_v13);
        instance.get_physical_device_features2(physical, &mut query);
        let mut extensions = Vec::new();
        if has(&available, portability_subset::NAME) {
            extensions.push(portability_subset::NAME.as_ptr());
        }
        if has(&available, push_descriptor::NAME) {
            extensions.push(push_descriptor::NAME.as_ptr());
        }
        let mut supported_copy = vk::PhysicalDeviceHostImageCopyFeaturesEXT::default();
        let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_copy);
        instance.get_physical_device_features2(physical, &mut query);
        let copies = has(&available, host_image_copy::NAME) && supported_copy.host_image_copy != 0;
        let mut copy = vk::PhysicalDeviceHostImageCopyFeaturesEXT::default().host_image_copy(true);
        if copies {
            extensions.push(host_image_copy::NAME.as_ptr());
        }
        let mut supported_matrix = vk::PhysicalDeviceCooperativeMatrixFeaturesKHR::default();
        let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_matrix);
        instance.get_physical_device_features2(physical, &mut query);
        let matrices =
            has(&available, cooperative_matrix::NAME) && supported_matrix.cooperative_matrix != 0;
        let mut matrix =
            vk::PhysicalDeviceCooperativeMatrixFeaturesKHR::default().cooperative_matrix(true);
        if matrices {
            extensions.push(cooperative_matrix::NAME.as_ptr());
        }
        let mut supported_wide = vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT::default();
        let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_wide);
        instance.get_physical_device_features2(physical, &mut query);
        let wide = has(&available, shader_image_atomic_int64::NAME)
            && supported_wide.shader_image_int64_atomics != 0;
        let mut image64 = vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT::default()
            .shader_image_int64_atomics(true);
        if wide {
            extensions.push(shader_image_atomic_int64::NAME.as_ptr());
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
        let mut query = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut supported_accel)
            .push_next(&mut supported_query);
        instance.get_physical_device_features2(physical, &mut query);
        let rays = has(&available, acceleration_structure::NAME)
            && has(&available, deferred_host_operations::NAME)
            && has(&available, ray_query::NAME)
            && supported_accel.acceleration_structure != 0
            && supported_query.ray_query != 0;
        let mut accel = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
            .acceleration_structure(true);
        let mut rq = vk::PhysicalDeviceRayQueryFeaturesKHR::default().ray_query(true);
        if rays {
            extensions.push(acceleration_structure::NAME.as_ptr());
            extensions.push(deferred_host_operations::NAME.as_ptr());
            extensions.push(ray_query::NAME.as_ptr());
        }
        if meshes {
            extensions.push(mesh_shader::NAME.as_ptr());
        }
        let supported = instance.get_physical_device_features(physical);
        let features = vk::PhysicalDeviceFeatures::default()
            .sampler_anisotropy(supported.sampler_anisotropy != 0)
            .image_cube_array(supported.image_cube_array != 0)
            .multi_viewport(supported.multi_viewport != 0)
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
            .shader_draw_parameters(supported_v11.shader_draw_parameters != 0);
        let mut supported_v12 = vk::PhysicalDeviceVulkan12Features::default();
        let mut query = vk::PhysicalDeviceFeatures2::default().push_next(&mut supported_v12);
        instance.get_physical_device_features2(physical, &mut query);
        let s = &supported_v12;
        let mut v12 = vk::PhysicalDeviceVulkan12Features::default()
            .buffer_device_address(s.buffer_device_address != 0)
            .timeline_semaphore(s.timeline_semaphore != 0)
            .descriptor_indexing(s.descriptor_indexing != 0)
            .runtime_descriptor_array(s.runtime_descriptor_array != 0)
            .descriptor_binding_partially_bound(s.descriptor_binding_partially_bound != 0)
            .descriptor_binding_sampled_image_update_after_bind(
                s.descriptor_binding_sampled_image_update_after_bind != 0,
            )
            .descriptor_binding_storage_image_update_after_bind(
                s.descriptor_binding_storage_image_update_after_bind != 0,
            )
            .shader_sampled_image_array_non_uniform_indexing(
                s.shader_sampled_image_array_non_uniform_indexing != 0,
            )
            .shader_storage_image_array_non_uniform_indexing(
                s.shader_storage_image_array_non_uniform_indexing != 0,
            )
            .descriptor_binding_storage_buffer_update_after_bind(
                supported_v12.descriptor_binding_storage_buffer_update_after_bind != 0,
            )
            .shader_storage_buffer_array_non_uniform_indexing(
                supported_v12.shader_storage_buffer_array_non_uniform_indexing != 0,
            )
            .shader_output_viewport_index(supported_v12.shader_output_viewport_index != 0)
            .shader_output_layer(supported_v12.shader_output_layer != 0)
            .draw_indirect_count(supported_v12.draw_indirect_count != 0)
            .vulkan_memory_model(supported_v12.vulkan_memory_model != 0)
            .vulkan_memory_model_device_scope(supported_v12.vulkan_memory_model_device_scope != 0)
            .shader_float16(supported_v12.shader_float16 != 0)
            .shader_int8(supported_v12.shader_int8 != 0)
            .shader_buffer_int64_atomics(supported_v12.shader_buffer_int64_atomics != 0)
            .shader_shared_int64_atomics(supported_v12.shader_shared_int64_atomics != 0)
            .sampler_mirror_clamp_to_edge(supported_v12.sampler_mirror_clamp_to_edge != 0);
        let mut v13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(supported_v13.synchronization2 != 0)
            .dynamic_rendering(supported_v13.dynamic_rendering != 0)
            .maintenance4(supported_v13.maintenance4 != 0);
        let info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&infos)
            .enabled_extension_names(&extensions)
            .enabled_features(&features)
            .push_next(&mut v11)
            .push_next(&mut v12)
            .push_next(&mut v13);
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
        let mut info = info;
        if rated {
            info = info.push_next(&mut rate);
        }
        if copies {
            info = info.push_next(&mut copy);
        }
        if matrices {
            info = info.push_next(&mut matrix);
        }
        if wide {
            info = info.push_next(&mut image64);
        }
        let raw = instance.create_device(physical, &info, None).unwrap();
        let mut enabled = vk::PhysicalDeviceFeatures2::default().features(features);
        enabled.p_next = info.p_next as *mut _;
        let device = mk::Device::adopt(
            &entry,
            instance.clone(),
            physical,
            raw.clone(),
            &slots,
            &enabled,
        )
        .unwrap();
        Some(Host {
            device: ManuallyDrop::new(device),
            raw,
            instance,
            entry,
        })
    }
}
