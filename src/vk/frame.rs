use ash::vk;

pub struct Frame {
    pub pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
}

impl Frame {
    pub fn new(device: &ash::Device, family: u32) -> Result<Self, vk::Result> {
        unsafe {
            let info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(family);
            let pool = device.create_command_pool(&info, None)?;
            let info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let buffer = match device.allocate_command_buffers(&info) {
                Ok(list) => list[0],
                Err(e) => {
                    device.destroy_command_pool(pool, None);
                    return Err(e);
                }
            };
            Ok(Self { pool, buffer })
        }
    }

    pub fn reset(&self, device: &ash::Device) -> Result<(), vk::Result> {
        unsafe { device.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty()) }
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_command_pool(self.pool, None);
        }
    }
}
