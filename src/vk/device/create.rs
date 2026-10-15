use super::Device;
use crate::Error;
use ash::ext::{
    debug_utils, host_image_copy, mesh_shader, rasterization_order_attachment_access,
    shader_image_atomic_int64,
};
use ash::khr::{
    dynamic_rendering_local_read, portability_enumeration, portability_subset, push_descriptor,
};
use ash::vk;
use std::ffi::{CStr, c_char};

fn has(list: &[vk::ExtensionProperties], name: &CStr) -> bool {
    list.iter().any(|e| e.extension_name_as_c_str() == Ok(name))
}

fn instance(entry: &ash::Entry) -> Result<ash::Instance, Error> {
    let available = unsafe { entry.enumerate_instance_extension_properties(None) }?;
    let mut names = Vec::new();
    let mut flags = vk::InstanceCreateFlags::empty();
    if has(&available, debug_utils::NAME) {
        names.push(debug_utils::NAME.as_ptr());
    }
    if has(&available, portability_enumeration::NAME) {
        names.push(portability_enumeration::NAME.as_ptr());
        flags |= vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR;
    }
    let app = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
    let info = vk::InstanceCreateInfo::default()
        .flags(flags)
        .application_info(&app)
        .enabled_extension_names(&names);
    Ok(unsafe { entry.create_instance(&info, None) }?)
}

fn physical(instance: &ash::Instance) -> Result<vk::PhysicalDevice, Error> {
    let list = unsafe { instance.enumerate_physical_devices() }?;
    let usable: Vec<_> = list
        .into_iter()
        .filter(|&p| {
            unsafe { instance.get_physical_device_properties(p) }.api_version >= vk::API_VERSION_1_3
        })
        .collect();
    let discrete = usable.iter().copied().find(|&p| {
        unsafe { instance.get_physical_device_properties(p) }.device_type
            == vk::PhysicalDeviceType::DISCRETE_GPU
    });
    discrete
        .or(usable.first().copied())
        .ok_or_else(|| Error::Unsupported("no Vulkan 1.3 device".into()))
}

fn slots(instance: &ash::Instance, physical: vk::PhysicalDevice, queues: u32) -> Vec<(u32, u32)> {
    let families = unsafe { instance.get_physical_device_queue_family_properties(physical) };
    let mut order: Vec<u32> = (0..families.len() as u32)
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
    order
        .into_iter()
        .flat_map(|f| (0..families[f as usize].queue_count).map(move |i| (f, i)))
        .take(queues as usize)
        .collect()
}

impl Device {
    pub fn create_device(queues: u32) -> Result<Self, Error> {
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| Error::Unsupported(format!("no Vulkan loader: {e}")))?;
        let instance = instance(&entry)?;
        match Self::open(&instance, queues) {
            Ok(mut device) => {
                device.entry = Some(entry);
                Ok(device)
            }
            Err(e) => {
                unsafe { instance.destroy_instance(None) };
                Err(e)
            }
        }
    }

    fn open(instance: &ash::Instance, queues: u32) -> Result<Self, Error> {
        let physical = physical(instance)?;
        let slots = slots(instance, physical, queues);
        let count = slots.len() as u32;
        if count < queues {
            return Err(Error::Unsupported(format!(
                "{queues} queues asked, the device has {count}"
            )));
        }
        let mut families: Vec<u32> = slots.iter().map(|s| s.0).collect();
        families.dedup();
        let priorities = vec![1.0; slots.len()];
        let infos: Vec<_> = families
            .iter()
            .map(|&f| {
                let count = slots.iter().filter(|s| s.0 == f).count();
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(f)
                    .queue_priorities(&priorities[..count])
            })
            .collect();
        let available = unsafe { instance.enumerate_device_extension_properties(physical) }?;
        let mut f = Features::new(instance, physical, &available);
        let names = f.names(&available);
        let info = f.chain(&infos, &names);
        let raw = unsafe { instance.create_device(physical, &info, None) }?;
        let next = info.p_next.cast_mut();
        let mut enabled = vk::PhysicalDeviceFeatures2::default().features(f.core);
        enabled.p_next = next;
        Self::import_device(instance.clone(), physical, raw.clone(), &slots, &enabled)
            .inspect_err(|_| unsafe { raw.destroy_device(None) })
    }
}

#[derive(Default)]
struct Features {
    core: vk::PhysicalDeviceFeatures,
    v11: vk::PhysicalDeviceVulkan11Features<'static>,
    v12: vk::PhysicalDeviceVulkan12Features<'static>,
    v13: vk::PhysicalDeviceVulkan13Features<'static>,
    mesh: vk::PhysicalDeviceMeshShaderFeaturesEXT<'static>,
    copy: vk::PhysicalDeviceHostImageCopyFeaturesEXT<'static>,
    local: vk::PhysicalDeviceDynamicRenderingLocalReadFeaturesKHR<'static>,
    order: vk::PhysicalDeviceRasterizationOrderAttachmentAccessFeaturesEXT<'static>,
    image64: vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT<'static>,
}

impl Features {
    fn new(
        instance: &ash::Instance,
        physical: vk::PhysicalDevice,
        available: &[vk::ExtensionProperties],
    ) -> Self {
        let mut s = Self::default();
        let mut query = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut s.v11)
            .push_next(&mut s.v12)
            .push_next(&mut s.v13);
        if has(available, mesh_shader::NAME) {
            query = query.push_next(&mut s.mesh);
        }
        if has(available, host_image_copy::NAME) {
            query = query.push_next(&mut s.copy);
        }
        if has(available, dynamic_rendering_local_read::NAME) {
            query = query.push_next(&mut s.local);
        }
        if has(available, rasterization_order_attachment_access::NAME) {
            query = query.push_next(&mut s.order);
        }
        if has(available, shader_image_atomic_int64::NAME) {
            query = query.push_next(&mut s.image64);
        }
        unsafe { instance.get_physical_device_features2(physical, &mut query) };
        let c = query.features;
        let (v11, v12, v13) = (s.v11, s.v12, s.v13);
        Self {
            core: vk::PhysicalDeviceFeatures {
                sampler_anisotropy: c.sampler_anisotropy,
                image_cube_array: c.image_cube_array,
                multi_viewport: c.multi_viewport,
                depth_clamp: c.depth_clamp,
                occlusion_query_precise: c.occlusion_query_precise,
                fill_mode_non_solid: c.fill_mode_non_solid,
                multi_draw_indirect: c.multi_draw_indirect,
                depth_bias_clamp: c.depth_bias_clamp,
                independent_blend: c.independent_blend,
                dual_src_blend: c.dual_src_blend,
                shader_int64: c.shader_int64,
                shader_storage_image_read_without_format: c
                    .shader_storage_image_read_without_format,
                shader_storage_image_write_without_format: c
                    .shader_storage_image_write_without_format,
                ..Default::default()
            },
            v11: vk::PhysicalDeviceVulkan11Features::default()
                .shader_draw_parameters(v11.shader_draw_parameters != 0),
            v12: vk::PhysicalDeviceVulkan12Features {
                buffer_device_address: v12.buffer_device_address,
                timeline_semaphore: v12.timeline_semaphore,
                runtime_descriptor_array: v12.runtime_descriptor_array,
                descriptor_binding_partially_bound: v12.descriptor_binding_partially_bound,
                descriptor_binding_sampled_image_update_after_bind: v12
                    .descriptor_binding_sampled_image_update_after_bind,
                descriptor_binding_storage_image_update_after_bind: v12
                    .descriptor_binding_storage_image_update_after_bind,
                shader_sampled_image_array_non_uniform_indexing: v12
                    .shader_sampled_image_array_non_uniform_indexing,
                shader_storage_image_array_non_uniform_indexing: v12
                    .shader_storage_image_array_non_uniform_indexing,
                uniform_buffer_standard_layout: v12.uniform_buffer_standard_layout,
                draw_indirect_count: v12.draw_indirect_count,
                shader_buffer_int64_atomics: v12.shader_buffer_int64_atomics,
                shader_shared_int64_atomics: v12.shader_shared_int64_atomics,
                vulkan_memory_model: v12.vulkan_memory_model,
                vulkan_memory_model_device_scope: v12.vulkan_memory_model_device_scope,
                sampler_mirror_clamp_to_edge: v12.sampler_mirror_clamp_to_edge,
                shader_output_viewport_index: v12.shader_output_viewport_index,
                ..Default::default()
            },
            v13: vk::PhysicalDeviceVulkan13Features {
                synchronization2: v13.synchronization2,
                dynamic_rendering: v13.dynamic_rendering,
                subgroup_size_control: v13.subgroup_size_control,
                compute_full_subgroups: v13.compute_full_subgroups,
                ..Default::default()
            },
            mesh: vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
                .mesh_shader(s.mesh.mesh_shader != 0)
                .task_shader(s.mesh.mesh_shader != 0 && s.mesh.task_shader != 0),
            copy: vk::PhysicalDeviceHostImageCopyFeaturesEXT::default()
                .host_image_copy(s.copy.host_image_copy != 0),
            local: vk::PhysicalDeviceDynamicRenderingLocalReadFeaturesKHR::default()
                .dynamic_rendering_local_read(s.local.dynamic_rendering_local_read != 0),
            order: vk::PhysicalDeviceRasterizationOrderAttachmentAccessFeaturesEXT::default()
                .rasterization_order_color_attachment_access(
                    s.order.rasterization_order_color_attachment_access != 0,
                ),
            image64: vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT::default()
                .shader_image_int64_atomics(s.image64.shader_image_int64_atomics != 0),
        }
    }

    fn names(&self, available: &[vk::ExtensionProperties]) -> Vec<*const c_char> {
        [
            (portability_subset::NAME, true),
            (push_descriptor::NAME, true),
            (mesh_shader::NAME, self.mesh.mesh_shader != 0),
            (host_image_copy::NAME, self.copy.host_image_copy != 0),
            (
                dynamic_rendering_local_read::NAME,
                self.local.dynamic_rendering_local_read != 0,
            ),
            (
                rasterization_order_attachment_access::NAME,
                self.order.rasterization_order_color_attachment_access != 0,
            ),
            (
                shader_image_atomic_int64::NAME,
                self.image64.shader_image_int64_atomics != 0,
            ),
        ]
        .into_iter()
        .filter(|&(name, on)| on && has(available, name))
        .map(|(name, _)| name.as_ptr())
        .collect()
    }

    fn chain<'a>(
        &'a mut self,
        queues: &'a [vk::DeviceQueueCreateInfo<'a>],
        names: &'a [*const c_char],
    ) -> vk::DeviceCreateInfo<'a> {
        let mut info = vk::DeviceCreateInfo::default()
            .queue_create_infos(queues)
            .enabled_extension_names(names)
            .enabled_features(&self.core)
            .push_next(&mut self.v11)
            .push_next(&mut self.v12)
            .push_next(&mut self.v13);
        if self.mesh.mesh_shader != 0 {
            info = info.push_next(&mut self.mesh);
        }
        if self.copy.host_image_copy != 0 {
            info = info.push_next(&mut self.copy);
        }
        if self.local.dynamic_rendering_local_read != 0 {
            info = info.push_next(&mut self.local);
        }
        if self.order.rasterization_order_color_attachment_access != 0 {
            info = info.push_next(&mut self.order);
        }
        if self.image64.shader_image_int64_atomics != 0 {
            info = info.push_next(&mut self.image64);
        }
        info
    }
}
