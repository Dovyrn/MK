use crate::vk::Callback;
use crate::vk::extensions::Extensions;
use crate::vk::frame::Frame;
use crate::vk::recorder::Recorder;
use crate::vk::resources::Resources;
use ash::vk;
use std::collections::VecDeque;

pub struct Queue {
    pub raw: vk::Queue,
    pub family: u32,
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
        ext: &Extensions,
        layout: vk::PipelineLayout,
        heap: vk::DescriptorSet,
        res: &Resources,
        (commands, natives): (&[u8], &[Callback]),
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
            ext,
            buffer: frame.buffer,
            layout,
            res,
            colors: Vec::new(),
            depth: None,
            stencil: vk::RenderingAttachmentInfo::default(),
            views: 0,
            natives,
            bound: (vk::Pipeline::null(), vk::Pipeline::null()),
            rate: None,
        }
        .run(heap, commands);
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
            .map(|&(e, v)| info(res.events.get(e).0, v))
            .collect();
        self.count += 1;
        let mut signals: Vec<_> = signals
            .iter()
            .map(|&(e, v)| info(res.events.get(e).0, v))
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

    pub fn done(&self, device: &ash::Device, value: u64) {
        let semaphores = [self.semaphore];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe { device.wait_semaphores(&info, u64::MAX).unwrap() };
    }

    pub fn sync(
        &mut self,
        device: &ash::Device,
        wait: vk::Semaphore,
        signal: vk::Semaphore,
    ) -> u64 {
        let info = |semaphore, value| {
            vk::SemaphoreSubmitInfo::default()
                .semaphore(semaphore)
                .value(value)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        };
        self.count += 1;
        let waits: Vec<_> = [wait]
            .into_iter()
            .filter(|w| *w != vk::Semaphore::null())
            .map(|w| info(w, 0))
            .collect();
        let mut signals = vec![info(self.semaphore, self.count)];
        if signal != vk::Semaphore::null() {
            signals.push(info(signal, 0));
        }
        let submit = vk::SubmitInfo2::default()
            .wait_semaphore_infos(&waits)
            .signal_semaphore_infos(&signals);
        unsafe {
            device
                .queue_submit2(self.raw, &[submit], vk::Fence::null())
                .unwrap()
        };
        self.count
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
