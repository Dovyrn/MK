use crate::vk::Device;
use ash::vk;
use std::sync::Arc;

pub struct BufferObject {
    pub raw: vk::Buffer,
    pub memory: Arc<vk::DeviceMemory>,
    pub address: u64,
    pub data: *mut u8,
    pub own: bool,
}

unsafe impl Send for BufferObject {}

unsafe impl Sync for BufferObject {}

impl BufferObject {
    pub fn free(&self, d: &Device) {
        if self.own {
            unsafe { d.raw.destroy_buffer(self.raw, None) };
        }
        if Arc::strong_count(&self.memory) == 1 {
            d.free_memory(*self.memory);
        }
    }
}
