use crate::vk::Device;
use ash::vk;
use std::rc::Rc;

pub struct BufferObject {
    pub raw: vk::Buffer,
    pub memory: Rc<vk::DeviceMemory>,
    pub address: u64,
    pub data: *mut u8,
    pub own: bool,
}

impl BufferObject {
    pub fn free(&self, d: &Device) {
        if self.own {
            unsafe { d.raw.destroy_buffer(self.raw, None) };
        }
        if Rc::strong_count(&self.memory) == 1 {
            d.free_memory(*self.memory);
        }
    }
}
