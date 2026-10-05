use ash::vk;

pub struct Frame {
    pub pool: vk::CommandPool,
    pub buffer: vk::CommandBuffer,
    pub sets: Vec<vk::DescriptorPool>,
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
            Ok(Self {
                pool,
                buffer,
                sets: Vec::new(),
            })
        }
    }

    pub fn reset(&self, device: &ash::Device) -> Result<(), vk::Result> {
        unsafe {
            for &pool in &self.sets {
                device.reset_descriptor_pool(pool, vk::DescriptorPoolResetFlags::empty())?;
            }
            device.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
        }
    }

    pub fn set(
        &mut self,
        device: &ash::Device,
        layout: vk::DescriptorSetLayout,
    ) -> Result<vk::DescriptorSet, vk::Result> {
        let layouts = [layout];
        if let Some(&pool) = self.sets.last() {
            let info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(pool)
                .set_layouts(&layouts);
            if let Ok(sets) = unsafe { device.allocate_descriptor_sets(&info) } {
                return Ok(sets[0]);
            }
        }
        let sizes = [vk::DescriptorPoolSize {
            ty: vk::DescriptorType::INPUT_ATTACHMENT,
            descriptor_count: 8 * 64,
        }];
        let info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(64)
            .pool_sizes(&sizes);
        let pool = unsafe { device.create_descriptor_pool(&info, None) }?;
        self.sets.push(pool);
        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);
        Ok(unsafe { device.allocate_descriptor_sets(&info) }?[0])
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            for &pool in &self.sets {
                device.destroy_descriptor_pool(pool, None);
            }
            device.destroy_command_pool(self.pool, None);
        }
    }
}
