use crate::Texture;
use ash::vk;
use std::sync::Mutex;

pub struct SwapchainObject {
    pub raw: vk::SwapchainKHR,
    pub queue: usize,
    pub textures: Vec<Texture>,
    pub presents: Vec<vk::Semaphore>,
    pub acquires: Mutex<Acquires>,
}

pub struct Acquires {
    pub spare: vk::Semaphore,
    pub owned: Vec<vk::Semaphore>,
    pub current: Option<(u32, vk::Semaphore)>,
}

impl Acquires {
    pub fn all(&self) -> impl Iterator<Item = vk::Semaphore> + '_ {
        std::iter::once(self.spare).chain(self.owned.iter().copied())
    }
}
