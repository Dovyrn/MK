use super::Device;
use crate::{DeviceInfo, Error};
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
    pub fn supported_features() -> Result<DeviceInfo, Error> {
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| Error::Unsupported(format!("no Vulkan loader: {e}")))?;
        let instance = instance(&entry)?;
        let have = physical(&instance).and_then(|physical| {
            let available = unsafe { instance.enumerate_device_extension_properties(physical) }?;
            let queues = slots(&instance, physical, u32::MAX).len() as u32;
            Ok(Features::query(&instance, physical, &available).supported(queues))
        });
        unsafe { instance.destroy_instance(None) };
        have
    }

    pub fn create_device(info: &DeviceInfo) -> Result<Self, Error> {
        let entry = unsafe { ash::Entry::load() }
            .map_err(|e| Error::Unsupported(format!("no Vulkan loader: {e}")))?;
        let instance = instance(&entry)?;
        match Self::open(&instance, info) {
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

    fn open(instance: &ash::Instance, wanted: &DeviceInfo) -> Result<Self, Error> {
        let queues = wanted.queues;
        let physical = physical(instance)?;
        let slots = slots(instance, physical, queues);
        let count = slots.len() as u32;
        if count < queues {
            return Err(Error::Unsupported(format!(
                "{queues} queues asked, the device has {count}"
            )));
        }
        let available = unsafe { instance.enumerate_device_extension_properties(physical) }?;
        wanted.assert_usable(&Features::query(instance, physical, &available).supported(count));
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
        let mut f = Features::enable(wanted);
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
    push: bool,
}

impl Features {
    fn query(
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
        s.core = query.features;
        s.push = has(available, push_descriptor::NAME);
        s
    }

    fn supported(&self, queues: u32) -> DeviceInfo {
        let (c, v11, v12, v13) = (&self.core, &self.v11, &self.v12, &self.v13);
        DeviceInfo {
            queues,
            addresses: v12.buffer_device_address != 0,
            timelines: v12.timeline_semaphore != 0,
            partially_bound: v12.descriptor_binding_partially_bound != 0,
            update_after_bind: v12.descriptor_binding_sampled_image_update_after_bind != 0
                && v12.descriptor_binding_storage_image_update_after_bind != 0,
            standard_layout: v12.uniform_buffer_standard_layout != 0,
            synchronization: v13.synchronization2 != 0,
            dynamic_rendering: v13.dynamic_rendering != 0,
            anisotropy: c.sampler_anisotropy != 0,
            cube_arrays: c.image_cube_array != 0,
            multi_viewport: c.multi_viewport != 0,
            depth_clamp: c.depth_clamp != 0,
            precise_occlusion: c.occlusion_query_precise != 0,
            line_fill: c.fill_mode_non_solid != 0,
            multi_draw: c.multi_draw_indirect != 0,
            bias_clamp: c.depth_bias_clamp != 0,
            independent_blend: c.independent_blend != 0,
            dual_blend: c.dual_src_blend != 0,
            int64: c.shader_int64 != 0,
            unformatted_reads: c.shader_storage_image_read_without_format != 0,
            unformatted_writes: c.shader_storage_image_write_without_format != 0,
            draw_parameters: v11.shader_draw_parameters != 0,
            runtime_arrays: v12.runtime_descriptor_array != 0,
            nonuniform: v12.shader_sampled_image_array_non_uniform_indexing != 0
                && v12.shader_storage_image_array_non_uniform_indexing != 0,
            indirect_count: v12.draw_indirect_count != 0,
            buffer_atomics64: v12.shader_buffer_int64_atomics != 0,
            shared_atomics64: v12.shader_shared_int64_atomics != 0,
            memory_model: v12.vulkan_memory_model != 0 && v12.vulkan_memory_model_device_scope != 0,
            mirror_clamp: v12.sampler_mirror_clamp_to_edge != 0,
            viewport_index: v12.shader_output_viewport_index != 0,
            subgroup_sizes: v13.subgroup_size_control != 0,
            full_subgroups: v13.compute_full_subgroups != 0,
            mesh: self.mesh.mesh_shader != 0,
            task: self.mesh.task_shader != 0,
            host_read: self.copy.host_image_copy != 0,
            attachment_reads: self.local.dynamic_rendering_local_read != 0,
            ordered_reads: self.order.rasterization_order_color_attachment_access != 0,
            image_atomics64: self.image64.shader_image_int64_atomics != 0,
            slot_binding: self.push,
        }
    }

    fn enable(info: &DeviceInfo) -> Self {
        let on = |b: bool| b as vk::Bool32;
        let mut e = Self::default();
        e.core.sampler_anisotropy = on(info.anisotropy);
        e.core.image_cube_array = on(info.cube_arrays);
        e.core.multi_viewport = on(info.multi_viewport);
        e.core.depth_clamp = on(info.depth_clamp);
        e.core.occlusion_query_precise = on(info.precise_occlusion);
        e.core.fill_mode_non_solid = on(info.line_fill);
        e.core.multi_draw_indirect = on(info.multi_draw);
        e.core.depth_bias_clamp = on(info.bias_clamp);
        e.core.independent_blend = on(info.independent_blend);
        e.core.dual_src_blend = on(info.dual_blend);
        e.core.shader_int64 = on(info.int64);
        e.core.shader_storage_image_read_without_format = on(info.unformatted_reads);
        e.core.shader_storage_image_write_without_format = on(info.unformatted_writes);
        e.v11.shader_draw_parameters = on(info.draw_parameters);
        let v12 = &mut e.v12;
        v12.buffer_device_address = on(info.addresses);
        v12.timeline_semaphore = on(info.timelines);
        v12.descriptor_binding_partially_bound = on(info.partially_bound);
        v12.descriptor_binding_sampled_image_update_after_bind = on(info.update_after_bind);
        v12.descriptor_binding_storage_image_update_after_bind = on(info.update_after_bind);
        v12.uniform_buffer_standard_layout = on(info.standard_layout);
        v12.runtime_descriptor_array = on(info.runtime_arrays);
        v12.shader_sampled_image_array_non_uniform_indexing = on(info.nonuniform);
        v12.shader_storage_image_array_non_uniform_indexing = on(info.nonuniform);
        v12.draw_indirect_count = on(info.indirect_count);
        v12.shader_buffer_int64_atomics = on(info.buffer_atomics64);
        v12.shader_shared_int64_atomics = on(info.shared_atomics64);
        v12.vulkan_memory_model = on(info.memory_model);
        v12.vulkan_memory_model_device_scope = on(info.memory_model);
        v12.sampler_mirror_clamp_to_edge = on(info.mirror_clamp);
        v12.shader_output_viewport_index = on(info.viewport_index);
        e.v13.synchronization2 = on(info.synchronization);
        e.v13.dynamic_rendering = on(info.dynamic_rendering);
        e.v13.subgroup_size_control = on(info.subgroup_sizes);
        e.v13.compute_full_subgroups = on(info.full_subgroups);
        e.mesh.mesh_shader = on(info.mesh);
        e.mesh.task_shader = on(info.task);
        e.copy.host_image_copy = on(info.host_read);
        e.local.dynamic_rendering_local_read = on(info.attachment_reads);
        e.order.rasterization_order_color_attachment_access = on(info.ordered_reads);
        e.image64.shader_image_int64_atomics = on(info.image_atomics64);
        e.push = info.slot_binding;
        e
    }

    fn names(&self, available: &[vk::ExtensionProperties]) -> Vec<*const c_char> {
        [
            (portability_subset::NAME, true),
            (push_descriptor::NAME, self.push),
            (
                mesh_shader::NAME,
                self.mesh.mesh_shader != 0 || self.mesh.task_shader != 0,
            ),
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
        if self.mesh.mesh_shader != 0 || self.mesh.task_shader != 0 {
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
