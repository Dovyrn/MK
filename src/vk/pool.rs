use ash::vk;
use std::cell::{Cell, RefCell};

pub struct Pool {
    pub raw: vk::CommandPool,
    buffers: RefCell<Vec<vk::CommandBuffer>>,
    used: Cell<usize>,
}

unsafe impl Sync for Pool {}

impl Pool {
    pub fn new(device: &ash::Device, family: u32) -> Result<Self, vk::Result> {
        let info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::TRANSIENT)
            .queue_family_index(family);
        Ok(Self {
            raw: unsafe { device.create_command_pool(&info, None) }?,
            buffers: RefCell::new(Vec::new()),
            used: Cell::new(0),
        })
    }

    pub fn next(&self, device: &ash::Device) -> Result<vk::CommandBuffer, vk::Result> {
        let mut buffers = self.buffers.borrow_mut();
        let i = self.used.get();
        if i == buffers.len() {
            let info = vk::CommandBufferAllocateInfo::default()
                .command_pool(self.raw)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            buffers.push(unsafe { device.allocate_command_buffers(&info) }?[0]);
        }
        self.used.set(i + 1);
        Ok(buffers[i])
    }

    pub fn reset(&self, device: &ash::Device) -> Result<(), vk::Result> {
        unsafe { device.reset_command_pool(self.raw, vk::CommandPoolResetFlags::empty()) }?;
        self.used.set(0);
        Ok(())
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_command_pool(self.raw, None) };
    }
}
