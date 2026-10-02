use ash::vk;

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub address: u64,
    pub data: *mut u8,
    pub own: bool,
    pub device: bool,
}

impl Buffer {
    pub fn free(&self, device: &ash::Device) {
        unsafe {
            if self.own {
                device.destroy_buffer(self.raw, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                device.free_memory(self.memory, None);
            }
        }
    }
}
