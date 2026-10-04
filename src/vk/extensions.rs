use crate::{Matrix, atomic, component};
use ash::ext::{debug_utils, host_image_copy, memory_budget, mesh_shader};
use ash::khr::push_descriptor;
use ash::khr::{acceleration_structure, cooperative_matrix, fragment_shading_rate};
use ash::vk;

pub struct Extensions {
    pub debug: Option<debug_utils::Device>,
    pub mesh: Option<mesh_shader::Device>,
    pub task: bool,
    pub sizes: bool,
    pub full: bool,
    pub atomics: u32,
    pub matrices: Vec<Matrix>,
    pub model: bool,
    pub moltenvk: bool,
    pub accel: Option<acceleration_structure::Device>,
    pub rates: Option<fragment_shading_rate::Device>,
    pub images: bool,
    pub features: vk::PhysicalDeviceFeatures,
    pub mirror: bool,
    pub count: bool,
    pub buffers: bool,
    pub push: Option<push_descriptor::Device>,
    pub copy: Option<host_image_copy::Device>,
    pub layouts: Vec<vk::ImageLayout>,
    pub budget: bool,
}

unsafe fn chained<'a, T: vk::TaggedStructure>(
    features: &'a vk::PhysicalDeviceFeatures2,
) -> Option<&'a T> {
    let mut next = features.p_next as *const vk::BaseInStructure;
    while let Some(s) = unsafe { next.as_ref() } {
        if s.s_type == T::STRUCTURE_TYPE {
            return Some(unsafe { &*(next as *const T) });
        }
        next = s.p_next;
    }
    None
}

impl Extensions {
    pub unsafe fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        physical: vk::PhysicalDevice,
        raw: &ash::Device,
        features: &vk::PhysicalDeviceFeatures2,
    ) -> Result<Self, String> {
        unsafe {
            let v12 = chained::<vk::PhysicalDeviceVulkan12Features>(features);
            let v13 = chained::<vk::PhysicalDeviceVulkan13Features>(features);
            let needs = [
                (
                    "VkPhysicalDeviceVulkan12Features.bufferDeviceAddress",
                    v12.map(|f| f.buffer_device_address),
                ),
                (
                    "VkPhysicalDeviceVulkan12Features.timelineSemaphore",
                    v12.map(|f| f.timeline_semaphore),
                ),
                (
                    "VkPhysicalDeviceVulkan12Features.descriptorBindingPartiallyBound",
                    v12.map(|f| f.descriptor_binding_partially_bound),
                ),
                (
                    "VkPhysicalDeviceVulkan12Features.descriptorBindingSampledImageUpdateAfterBind",
                    v12.map(|f| f.descriptor_binding_sampled_image_update_after_bind),
                ),
                (
                    "VkPhysicalDeviceVulkan12Features.descriptorBindingStorageImageUpdateAfterBind",
                    v12.map(|f| f.descriptor_binding_storage_image_update_after_bind),
                ),
                (
                    "VkPhysicalDeviceVulkan13Features.synchronization2",
                    v13.map(|f| f.synchronization2),
                ),
                (
                    "VkPhysicalDeviceVulkan13Features.dynamicRendering",
                    v13.map(|f| f.dynamic_rendering),
                ),
            ];
            let missing: Vec<_> = needs
                .iter()
                .filter(|(_, on)| on.unwrap_or(0) == 0)
                .map(|(name, _)| *name)
                .collect();
            if !missing.is_empty() {
                return Err(format!("adopt needs {}", missing.join(", ")));
            }
            let has = |name: &std::ffi::CStr| {
                (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr()).is_some()
            };
            let debug = has(c"vkCmdBeginDebugUtilsLabelEXT")
                .then(|| debug_utils::Device::new(instance, raw));
            let mesh = chained::<vk::PhysicalDeviceMeshShaderFeaturesEXT>(features)
                .is_some_and(|f| f.mesh_shader != 0)
                && has(c"vkCmdDrawMeshTasksEXT");
            let task = mesh
                && chained::<vk::PhysicalDeviceMeshShaderFeaturesEXT>(features)
                    .is_some_and(|f| f.task_shader != 0);
            let mesh = mesh.then(|| mesh_shader::Device::new(instance, raw));
            let accel = chained::<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>(features)
                .is_some_and(|f| f.acceleration_structure != 0)
                && has(c"vkCmdBuildAccelerationStructuresKHR");
            let accel = accel.then(|| acceleration_structure::Device::new(instance, raw));
            let rates = chained::<vk::PhysicalDeviceFragmentShadingRateFeaturesKHR>(features)
                .is_some_and(|f| f.pipeline_fragment_shading_rate != 0)
                && has(c"vkCmdSetFragmentShadingRateKHR");
            let images = rates
                && chained::<vk::PhysicalDeviceFragmentShadingRateFeaturesKHR>(features)
                    .is_some_and(|f| f.attachment_fragment_shading_rate != 0);
            let rates = rates.then(|| fragment_shading_rate::Device::new(instance, raw));
            let push = has(c"vkCmdPushDescriptorSetKHR")
                .then(|| push_descriptor::Device::new(instance, raw));
            let buffers =
                v12.is_some_and(|f| f.descriptor_binding_storage_buffer_update_after_bind != 0);
            let mut driver = vk::PhysicalDeviceDriverProperties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut driver);
            instance.get_physical_device_properties2(physical, &mut properties);
            let moltenvk = driver.driver_id == vk::DriverId::MOLTENVK;
            let copy = chained::<vk::PhysicalDeviceHostImageCopyFeaturesEXT>(features)
                .is_some_and(|f| f.host_image_copy != 0)
                && has(c"vkCopyMemoryToImageEXT")
                && !moltenvk;
            let layouts = if copy {
                let mut counts = vk::PhysicalDeviceHostImageCopyPropertiesEXT::default();
                let mut properties =
                    vk::PhysicalDeviceProperties2::default().push_next(&mut counts);
                instance.get_physical_device_properties2(physical, &mut properties);
                let mut src =
                    vec![vk::ImageLayout::UNDEFINED; counts.copy_src_layout_count as usize];
                let mut dst =
                    vec![vk::ImageLayout::UNDEFINED; counts.copy_dst_layout_count as usize];
                let mut lists = vk::PhysicalDeviceHostImageCopyPropertiesEXT::default()
                    .copy_src_layouts(&mut src)
                    .copy_dst_layouts(&mut dst);
                let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut lists);
                instance.get_physical_device_properties2(physical, &mut properties);
                src.into_iter().filter(|l| dst.contains(l)).collect()
            } else {
                Vec::new()
            };
            let copy = layouts
                .contains(&vk::ImageLayout::GENERAL)
                .then(|| host_image_copy::Device::new(instance, raw));
            let budget = instance
                .enumerate_device_extension_properties(physical)
                .unwrap_or_default()
                .iter()
                .any(|e| e.extension_name_as_c_str() == Ok(memory_budget::NAME));

            Ok(Self {
                debug,
                mesh,
                task,
                sizes: v13.is_some_and(|f| f.subgroup_size_control != 0),
                full: v13.is_some_and(|f| f.compute_full_subgroups != 0),
                atomics: v12.map_or(0, |f| {
                    let buffer = if f.shader_buffer_int64_atomics != 0 {
                        atomic::MIN_MAX | atomic::ALL | atomic::SIGNED
                    } else {
                        0
                    };
                    let shared = if f.shader_shared_int64_atomics != 0 {
                        atomic::SHARED
                    } else {
                        0
                    };
                    buffer | shared
                }) | if chained::<vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT>(
                    features,
                )
                .is_some_and(|f| f.shader_image_int64_atomics != 0)
                {
                    atomic::IMAGE
                } else {
                    0
                },
                matrices: if chained::<vk::PhysicalDeviceCooperativeMatrixFeaturesKHR>(features)
                    .is_some_and(|f| f.cooperative_matrix != 0)
                {
                    matrices(entry, instance, physical)
                } else {
                    Vec::new()
                },
                model: v12.is_some_and(|f| f.vulkan_memory_model != 0),
                moltenvk,
                accel,
                rates,
                images,
                features: features.features,
                mirror: v12.is_some_and(|f| f.sampler_mirror_clamp_to_edge != 0),
                count: v12.is_some_and(|f| f.draw_indirect_count != 0),
                buffers,
                push,
                copy,
                layouts,
                budget,
            })
        }
    }
}

fn kind(t: vk::ComponentTypeKHR) -> Option<u32> {
    Some(match t.as_raw() {
        0 => component::F16,
        1 => component::F32,
        2 => component::F64,
        3 => component::S8,
        4 => component::S16,
        5 => component::S32,
        6 => component::S64,
        7 => component::U8,
        8 => component::U16,
        9 => component::U32,
        10 => component::U64,
        1000141000 => component::BF16,
        1000491002 => component::E4M3,
        1000491003 => component::E5M2,
        _ => return None,
    })
}

unsafe fn matrices(
    entry: &ash::Entry,
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
) -> Vec<Matrix> {
    let loader = cooperative_matrix::Instance::new(entry, instance);
    let list = unsafe { loader.get_physical_device_cooperative_matrix_properties(physical) };
    list.unwrap_or_default()
        .into_iter()
        .filter(|p| p.scope == vk::ScopeKHR::SUBGROUP)
        .filter_map(|p| {
            Some(Matrix {
                m: p.m_size,
                n: p.n_size,
                k: p.k_size,
                a: kind(p.a_type)?,
                b: kind(p.b_type)?,
                c: kind(p.c_type)?,
                result: kind(p.result_type)?,
                saturate: p.saturating_accumulation != 0,
            })
        })
        .collect()
}
