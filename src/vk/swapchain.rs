use crate::Texture;
use ash::vk;
use std::sync::Mutex;

pub struct SwapchainObject {
    pub raw: vk::SwapchainKHR,
    pub queue: usize,
    pub textures: Vec<Texture>,
    pub acquires: Vec<vk::Semaphore>,
    pub presents: Vec<vk::Semaphore>,
    pub state: Mutex<(usize, Option<(u32, usize)>)>,
}
