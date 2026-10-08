use crate::Error;
use crate::handle::TimelineId;
use crate::vk::resources::Resources;
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

    pub fn submit(
        &self,
        device: &ash::Device,
        buffer: vk::CommandBuffer,
        res: &Resources,
        waits: &[(TimelineId, u64)],
        signals: &[(TimelineId, u64)],
    ) -> Result<(), Error> {
        unsafe { device.end_command_buffer(buffer) }.map_err(Error::from)?;
        let info = |semaphore, value| {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore)
                .value(value)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        };
        let waits: Vec<_> = waits
            .iter()
            .map(|&(t, v)| info(res.timelines.get(t).raw, v))
            .collect();
        let signals: Vec<_> = signals
            .iter()
            .map(|&(t, v)| info(res.timelines.get(t).raw, v))
            .collect();
        let buffers = [vk::CommandBufferSubmitInfo::default().command_buffer(buffer)];
        let submit = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&waits)
            .command_buffer_infos(&buffers)
            .signal_semaphore_infos(&signals);
        unsafe { device.queue_submit2(self.raw, &[submit], vk::Fence::null()) }.map_err(Error::from)
    }
}
