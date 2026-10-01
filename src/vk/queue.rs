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

pub fn timeline(device: &ash::Device) -> Result<vk::Semaphore, vk::Result> {
    let mut kind =
        vk::SemaphoreTypeCreateInfo::default().semaphore_type(vk::SemaphoreType::TIMELINE);
    let info = vk::SemaphoreCreateInfo::default().push_next(&mut kind);
    unsafe { device.create_semaphore(&info, None) }
}

impl Queue {
    pub fn new(device: &ash::Device, family: u32, index: u32) -> Result<Self, String> {
        Ok(Self {
            raw: unsafe { device.get_device_queue(family, index) },
            family,
            semaphore: timeline(device).map_err(|e| e.to_string())?,
            count: 0,
            free: Vec::new(),
            busy: VecDeque::new(),
        })
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
    ) -> Result<(), String> {
        let done = unsafe { device.get_semaphore_counter_value(self.semaphore) }
            .map_err(|e| e.to_string())?;
        while let Some((frame, _)) = self.busy.pop_front_if(|(_, v)| *v <= done) {
            frame.reset(device).map_err(|e| e.to_string())?;
            self.free.push(frame);
        }
        let frame = match self.free.pop() {
            Some(f) => f,
            None => Frame::new(device, self.family).map_err(|e| e.to_string())?,
        };
        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        if let Err(e) = unsafe { device.begin_command_buffer(frame.buffer, &begin) } {
            self.free.push(frame);
            return Err(e.to_string());
        }
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
            scope: "",
        }
        .run(heap, commands);
        let ended = unsafe { device.end_command_buffer(frame.buffer) };
        if let Err(e) = result
            .map_err(String::from)
            .and(ended.map_err(|e| e.to_string()))
        {
            frame.reset(device).map_err(|e| e.to_string())?;
            self.free.push(frame);
            return Err(e);
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
        if let Err(e) = unsafe { device.queue_submit2(self.raw, &[submit], vk::Fence::null()) } {
            self.count -= 1;
            frame.reset(device).map_err(|e| e.to_string())?;
            self.free.push(frame);
            return Err(e.to_string());
        }
        self.busy.push_back((frame, self.count));
        Ok(())
    }

    pub fn done(&self, device: &ash::Device, value: u64) -> Result<(), String> {
        let semaphores = [self.semaphore];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe { device.wait_semaphores(&info, u64::MAX) }.map_err(|e| e.to_string())
    }

    pub fn sync(
        &mut self,
        device: &ash::Device,
        wait: vk::Semaphore,
        signal: vk::Semaphore,
    ) -> Result<u64, String> {
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
        if let Err(e) = unsafe { device.queue_submit2(self.raw, &[submit], vk::Fence::null()) } {
            self.count -= 1;
            return Err(e.to_string());
        }
        Ok(self.count)
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
