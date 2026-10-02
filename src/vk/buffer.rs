use ash::vk;

pub struct Buffer {
    pub raw: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub address: u64,
    pub data: *mut u8,
    pub own: bool,
    pub device: bool,
}
