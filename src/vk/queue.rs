use crate::slots::Slots;
use crate::vk::buffer::Buffer;
use crate::vk::frame::Frame;
use crate::vk::recorder::Recorder;
use ash::ext::debug_utils;
use ash::vk;
use std::collections::VecDeque;

pub struct Queue {
    raw: vk::Queue,
    family: u32,
    semaphore: vk::Semaphore,
    count: u64,
    free: Vec<Frame>,
    busy: VecDeque<(Frame, u64)>,
}

pub fn timeline(device: &ash::Device) -> vk::Semaphore {
    let mut kind =
        vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
    let info = vk::SemaphoreCreateInfo::default().push_next(&mut kind);
    unsafe { device.create_semaphore(&info, None).unwrap() }
}

impl Queue {
    pub fn new(device: &ash::Device, family: u32, index: u32) -> Self {
        Self {
            raw: unsafe { device.get_device_queue(family, index) },
            family,
            semaphore: timeline(device),
            count: 0,
            free: Vec::new(),
            busy: VecDeque::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn submit(
        &mut self,
        device: &ash::Device,
        debug: Option<&debug_utils::Device>,
        layout: vk::PipelineLayout,
        buffers: &Slots<Buffer>,
        pipelines: &Slots<vk::Pipeline>,
        events: &Slots<vk::Semaphore>,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), &'static str> {
        let done = unsafe { device.get_semaphore_counter_value(self.semaphore).unwrap() };
        while self.busy.front().is_some_and(|(_, v)| *v <= done) {
            let (frame, _) = self.busy.pop_front().unwrap();
            frame.reset(device);
            self.free.push(frame);
        }
        let frame = self
            .free
            .pop()
            .unwrap_or_else(|| Frame::new(device, self.family));
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { device.begin_command_buffer(frame.buffer, &begin).unwrap() };
        let result = Recorder {
            device,
            debug,
            buffer: frame.buffer,
            layout,
            buffers,
            pipelines,
        }
        .run(commands);
        unsafe { device.end_command_buffer(frame.buffer).unwrap() };
        if let Err(name) = result {
            frame.reset(device);
            self.free.push(frame);
            return Err(name);
        }
        let info = |semaphore, value| {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore)
                .value(value)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        };
        let waits: Vec<_> = waits
            .iter()
            .map(|&(e, v)| info(*events.get(e), v))
            .collect();
        self.count += 1;
        let mut signals: Vec<_> = signals
            .iter()
            .map(|&(e, v)| info(*events.get(e), v))
            .collect();
        signals.push(info(self.semaphore, self.count));
        let buffers = [vk::CommandBufferSubmitInfo::default().command_buffer(frame.buffer)];
        let submit = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&waits)
            .command_buffer_infos(&buffers)
            .signal_semaphore_infos(&signals);
        unsafe {
            device
                .queue_submit2(self.raw, &[submit], vk::Fence::null())
                .unwrap()
        };
        self.busy.push_back((frame, self.count));
        Ok(())
    }

    pub fn destroy(&mut self, device: &ash::Device) {
        for frame in self.free.drain(..) {
            frame.destroy(device);
        }
        for (frame, _) in self.busy.drain(..) {
            frame.destroy(device);
        }
        unsafe { device.destroy_semaphore(self.semaphore, None) };
    }
}
