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
    pub push: Option<push_descriptor::Device>,
}
