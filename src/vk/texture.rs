use crate::format;
use crate::vk::Device;
use ash::vk;
use std::rc::Rc;

pub struct TextureObject {
    pub image: vk::Image,
    pub owner: Option<Rc<vk::DeviceMemory>>,
    pub heap: Option<Rc<vk::DeviceMemory>>,
    pub view: vk::ImageView,
    pub target: vk::ImageView,
    pub aspect: vk::ImageAspectFlags,
    pub format: u32,
    pub usage: u32,
    pub base_mip: u32,
    pub mips: u32,
    pub base_layer: u32,
    pub layers: u32,
}

impl TextureObject {
    pub fn free(self, d: &Device) {
        unsafe {
            d.raw.destroy_image_view(self.view, None);
            d.raw.destroy_image_view(self.target, None);
        }
        if let Some(memory) = self.owner.and_then(Rc::into_inner) {
            unsafe { d.raw.destroy_image(self.image, None) };
            d.free_memory(memory);
        }
        if let Some(memory) = self.heap.and_then(Rc::into_inner) {
            d.free_memory(memory);
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
