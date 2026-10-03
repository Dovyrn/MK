use ash::vk;
use std::rc::Rc;

pub struct Heap {
    pub memory: Rc<vk::DeviceMemory>,
    pub data: *mut u8,
    pub shared: bool,
    pub device: bool,
    pub index: u32,
}
