use ash::ext::{debug_utils, mesh_shader};
use ash::khr::{acceleration_structure, surface, swapchain};

pub struct Extensions {
    pub debug: Option<debug_utils::Device>,
    pub mesh: Option<mesh_shader::Device>,
    pub ray: Option<acceleration_structure::Device>,
    pub surfaces: Option<(surface::Instance, swapchain::Device)>,
}
