use crate::format;
use ash::vk;

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub aspect: vk::ImageAspectFlags,
    pub own: bool,
}

pub fn aspect(f: u32) -> vk::ImageAspectFlags {
    match f {
        format::D16_UNORM | format::D32_FLOAT => vk::ImageAspectFlags::DEPTH,
        format::D24_UNORM_S8 | format::D32_FLOAT_S8 => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        format::S8_UINT => vk::ImageAspectFlags::STENCIL,
        _ => vk::ImageAspectFlags::COLOR,
    }
}
