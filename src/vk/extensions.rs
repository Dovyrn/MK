use crate::vk::ffi;
use crate::vk::generated::Generated;
use ash::ext::{conditional_rendering, debug_utils, external_memory_host, mesh_shader};
use ash::khr::push_descriptor;
use ash::khr::{
    acceleration_structure, external_memory_fd, fragment_shading_rate, ray_tracing_pipeline,
};
use ash::vk;

pub struct Extensions {
    pub debug: Option<debug_utils::Device>,
    pub mesh: Option<mesh_shader::Device>,
    pub ray: Option<acceleration_structure::Device>,
    pub rays: Option<ray_tracing_pipeline::Device>,
    pub rates: Option<fragment_shading_rate::Device>,
    pub host: Option<external_memory_host::Device>,
    pub fds: Option<external_memory_fd::Device>,
    pub conditional: Option<conditional_rendering::Device>,
    pub generated: Option<Generated>,
    pub features: vk::PhysicalDeviceFeatures,
    pub multiview: bool,
    pub buffers: bool,
    pub push: Option<push_descriptor::Device>,
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
        instance: &ash::Instance,
        raw: &ash::Device,
        features: &vk::PhysicalDeviceFeatures2,
    ) -> Self {
        unsafe {
            let has = |name: &std::ffi::CStr| {
                (instance.fp_v1_0().get_device_proc_addr)(raw.handle(), name.as_ptr()).is_some()
            };
            let debug = has(c"vkCmdBeginDebugUtilsLabelEXT")
                .then(|| debug_utils::Device::new(instance, raw));
            let mesh = chained::<vk::PhysicalDeviceMeshShaderFeaturesEXT>(features)
                .is_some_and(|f| f.mesh_shader != 0)
                && has(c"vkCmdDrawMeshTasksEXT");
            let mesh = mesh.then(|| mesh_shader::Device::new(instance, raw));
            let ray = chained::<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>(features)
                .is_some_and(|f| f.acceleration_structure != 0)
                && has(c"vkCmdBuildAccelerationStructuresKHR");
            let ray = ray.then(|| acceleration_structure::Device::new(instance, raw));
            let generated = chained::<ffi::Features>(features).is_some_and(|f| f.generated != 0);
            let generated = generated.then(|| Generated::load(instance, raw)).flatten();
            let rays = chained::<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>(features)
                .is_some_and(|f| f.ray_tracing_pipeline != 0)
                && has(c"vkCreateRayTracingPipelinesKHR");
            let rays = rays.then(|| ray_tracing_pipeline::Device::new(instance, raw));
            let host = has(c"vkGetMemoryHostPointerPropertiesEXT")
                .then(|| external_memory_host::Device::new(instance, raw));
            let conditional =
                chained::<vk::PhysicalDeviceConditionalRenderingFeaturesEXT>(features)
                    .is_some_and(|f| f.conditional_rendering != 0)
                    && has(c"vkCmdBeginConditionalRenderingEXT");
            let conditional =
                conditional.then(|| conditional_rendering::Device::new(instance, raw));
            let fds =
                has(c"vkGetMemoryFdKHR").then(|| external_memory_fd::Device::new(instance, raw));
            let rates = chained::<vk::PhysicalDeviceFragmentShadingRateFeaturesKHR>(features)
                .is_some_and(|f| f.pipeline_fragment_shading_rate != 0)
                && has(c"vkCmdSetFragmentShadingRateKHR");
            let rates = rates.then(|| fragment_shading_rate::Device::new(instance, raw));
            let push = has(c"vkCmdPushDescriptorSetKHR")
                .then(|| push_descriptor::Device::new(instance, raw));
            let multiview = chained::<vk::PhysicalDeviceVulkan11Features>(features)
                .is_some_and(|f| f.multiview != 0)
                || chained::<vk::PhysicalDeviceMultiviewFeatures>(features)
                    .is_some_and(|f| f.multiview != 0);
            let buffers = chained::<vk::PhysicalDeviceVulkan12Features>(features)
                .is_some_and(|f| f.descriptor_binding_storage_buffer_update_after_bind != 0);
            Self {
                debug,
                mesh,
                ray,
                rays,
                rates,
                host,
                fds,
                conditional,
                generated,
                features: features.features,
                multiview,
                buffers,
                push,
            }
        }
    }
}
