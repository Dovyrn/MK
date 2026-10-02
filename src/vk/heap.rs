use ash::vk;

pub struct Heap {
    pub memory: vk::DeviceMemory,
    pub data: *mut u8,
    pub shared: bool,
    pub device: bool,
}
