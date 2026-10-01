use crate::vk::generated::Generated;
use ash::ext::{debug_utils, external_memory_host, mesh_shader};
use ash::khr::{
    acceleration_structure, fragment_shading_rate, ray_tracing_pipeline, surface, swapchain,
};

pub struct Extensions {
    pub debug: Option<debug_utils::Device>,
    pub mesh: Option<mesh_shader::Device>,
    pub ray: Option<acceleration_structure::Device>,
    pub rays: Option<ray_tracing_pipeline::Device>,
    pub rates: Option<fragment_shading_rate::Device>,
    pub host: Option<external_memory_host::Device>,
    pub generated: Option<Generated>,
    pub surfaces: Option<(surface::Instance, swapchain::Device)>,
}
