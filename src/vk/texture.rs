use crate::Format;
use crate::vk::Device;
use ash::vk;
use std::sync::Arc;

pub struct TextureObject {
    pub image: vk::Image,
    pub owner: Option<Arc<vk::DeviceMemory>>,
    pub heap: Option<Arc<vk::DeviceMemory>>,
    pub view: vk::ImageView,
    pub target: vk::ImageView,
    pub aspect: vk::ImageAspectFlags,
    pub format: crate::Format,
    pub usage: crate::TextureUsage,
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
        if let Some(memory) = self.owner.and_then(Arc::into_inner) {
            unsafe { d.raw.destroy_image(self.image, None) };
            d.free_memory(memory);
        }
        if let Some(memory) = self.heap.and_then(Arc::into_inner) {
            d.free_memory(memory);
        }
    }
}

pub fn aspect(f: crate::Format) -> vk::ImageAspectFlags {
    match f {
        Format::D16Unorm | Format::D32Float => vk::ImageAspectFlags::DEPTH,
        Format::D24UnormS8 | Format::D32FloatS8 => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        Format::S8Uint => vk::ImageAspectFlags::STENCIL,
        _ => vk::ImageAspectFlags::COLOR,
    }
}

pub fn count(n: u32, base: u32, total: u32) -> u32 {
    if n == !0 { total - base } else { n }
}
