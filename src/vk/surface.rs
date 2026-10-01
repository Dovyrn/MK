use ash::vk;

pub struct Surface {
    pub raw: vk::SurfaceKHR,
    pub own: bool,
    pub swapchain: vk::SwapchainKHR,
    pub format: u32,
    pub vsync: bool,
    pub count: u32,
    pub usage: u32,
    pub size: [u32; 2],
    pub suboptimal: bool,
    pub images: Vec<u32>,
    pub acquires: Vec<(vk::Semaphore, u64)>,
    pub presents: Vec<vk::Semaphore>,
    pub next: usize,
    pub index: u32,
}
