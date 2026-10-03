use crate::vk::Device;
use ash::vk;
use std::rc::Rc;

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: Rc<vk::DeviceMemory>,
    pub address: u64,
    pub data: *mut u8,
    pub own: bool,
    pub device: bool,
}

impl Buffer {
    pub fn free(&self, d: &Device) {
        if self.own {
            unsafe { d.raw.destroy_buffer(self.raw, None) };
        }
        if Rc::strong_count(&self.memory) == 1 {
            d.release(*self.memory);
        }
    }
}
