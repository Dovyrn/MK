use crate::format;
use ash::vk;

pub struct Texture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub target: vk::ImageView,
    pub aspect: vk::ImageAspectFlags,
    pub format: u32,
    pub usage: u32,
    pub base_mip: u32,
    pub mips: u32,
    pub base_layer: u32,
    pub layers: u32,
    pub own: bool,
    pub tile: vk::Extent3D,
}

impl Texture {
    pub fn free(&self, device: &ash::Device) {
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image_view(self.target, None);
            if self.own {
                device.destroy_image(self.image, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                device.free_memory(self.memory, None);
            }
        }
    }
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

pub fn count(n: u32, base: u32, total: u32) -> u32 {
    if n == !0 { total - base } else { n }
}
