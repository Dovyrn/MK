use crate::Texture;
use ash::vk;
use std::sync::Mutex;

pub struct SwapchainObject {
    pub queue: usize,
    pub surface: vk::SurfaceKHR,
    pub state: Mutex<State>,
}

#[derive(Default)]
pub struct State {
    pub raw: vk::SwapchainKHR,
    pub textures: Vec<Texture>,
    pub presents: Vec<vk::Semaphore>,
    pub spare: vk::Semaphore,
    pub owned: Vec<vk::Semaphore>,
    pub current: Option<(u32, vk::Semaphore)>,
    pub suboptimal: bool,
    pub count: u64,
    pub mark: Option<u64>,
}

impl State {
    pub fn semaphores(&self) -> impl Iterator<Item = vk::Semaphore> + '_ {
        std::iter::once(self.spare)
            .chain(self.owned.iter().copied())
            .chain(self.presents.iter().copied())
    }
}

pub struct Present {
    pub raw: vk::SwapchainKHR,
    pub image: u32,
    pub acquire: vk::Semaphore,
    pub signal: vk::Semaphore,
    pub id: u64,
}
