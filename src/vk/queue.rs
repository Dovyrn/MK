use ash::vk;

pub const COMPUTE: [vk::PipelineBindPoint; 1] = [vk::PipelineBindPoint::COMPUTE];

pub const GRAPHICS: [vk::PipelineBindPoint; 2] = [
    vk::PipelineBindPoint::COMPUTE,
    vk::PipelineBindPoint::GRAPHICS,
];

pub struct Queue {
    pub raw: vk::Queue,
    pub family: u32,
    pub graphics: bool,
}

pub fn timeline(device: &ash::Device) -> Result<vk::Semaphore, vk::Result> {
    let mut kind =
        vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
    let info = vk::SemaphoreCreateInfo::default().push_next(&mut kind);
    unsafe { device.create_semaphore(&info, None) }
}

impl Queue {
    pub fn new(device: &ash::Device, family: u32, index: u32, graphics: bool) -> Self {
        Self {
            raw: unsafe { device.get_device_queue(family, index) },
            family,
            graphics,
        }
    }

    pub fn bind_points(&self) -> &'static [vk::PipelineBindPoint] {
        if self.graphics { &GRAPHICS } else { &COMPUTE }
    }
}
