use ash::vk;

pub struct Semaphore {
    pub raw: vk::Semaphore,
    pub own: bool,
}

impl Semaphore {
    pub fn free(&self, device: &ash::Device) {
        if self.own {
            unsafe { device.destroy_semaphore(self.raw, None) };
        }
    }
}
