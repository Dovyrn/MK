use ash::vk;

pub struct Event {
    pub raw: vk::Semaphore,
    pub own: bool,
}

impl Event {
    pub fn free(&self, device: &ash::Device) {
        if self.own {
            unsafe { device.destroy_semaphore(self.raw, None) };
        }
    }
}
