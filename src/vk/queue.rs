use crate::vk::Callback;
use crate::vk::extensions::Extensions;
use crate::vk::frame::Frame;
use crate::vk::recorder::Recorder;
use crate::vk::resources::Resources;
use ash::vk;
use std::collections::{BTreeMap, VecDeque};
use std::panic::AssertUnwindSafe;

pub struct Queue {
    pub raw: vk::Queue,
    pub family: u32,
    graphics: bool,
    pub semaphore: vk::Semaphore,
    pub count: u64,
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
    pub fn new(
        device: &ash::Device,
        family: u32,
        index: u32,
        graphics: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            raw: unsafe { device.get_device_queue(family, index) },
            family,
            graphics,
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
        let value = self.count + 1;
        let mut points = vec![vk::PipelineBindPoint::COMPUTE];
        if self.graphics {
            points.push(vk::PipelineBindPoint::GRAPHICS);
        }
        let run = || -> Result<(), String> {
            let begin = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            unsafe { device.begin_command_buffer(frame.buffer, &begin) }
                .map_err(|e| e.to_string())?;
            Recorder {
                device,
                ext,
                buffer: frame.buffer,
                layout,
                points: &points,
                res,
                colors: Vec::new(),
                depth: None,
                stencil: vk::RenderingAttachmentInfo::default(),
                natives,
                heap,
                compute: vk::Pipeline::null(),
                graphics: vk::Pipeline::null(),
                slots: BTreeMap::new(),
                push: Vec::new(),
                rate: None,
                defaults: false,
                area: vk::Rect2D::default(),
            }
            .run(commands)?;
            unsafe { device.end_command_buffer(frame.buffer) }.map_err(|e| e.to_string())?;
            let info = |semaphore, value| {
                vk::SemaphoreSubmitInfo::default()
                    .semaphore(semaphore)
                    .value(value)
                    .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            };
            let waits: Vec<_> = waits
                .iter()
                .map(|&(e, v)| info(res.events.get(e).raw, v))
                .collect();
            let mut signals: Vec<_> = signals
                .iter()
                .map(|&(e, v)| info(res.events.get(e).raw, v))
                .collect();
            signals.push(info(self.semaphore, value));
            let buffers = [vk::CommandBufferSubmitInfo::default().command_buffer(frame.buffer)];
            let submit = vk::SubmitInfo2::default()
                .wait_semaphore_infos(&waits)
                .command_buffer_infos(&buffers)
                .signal_semaphore_infos(&signals);
            unsafe { device.queue_submit2(self.raw, &[submit], vk::Fence::null()) }
                .map_err(|e| e.to_string())
        };
        match std::panic::catch_unwind(AssertUnwindSafe(run)) {
            Ok(Ok(())) => {
                self.count = value;
                self.busy.push_back((frame, value));
                Ok(())
            }
            Ok(Err(e)) => {
                frame.destroy(device);
                Err(e)
            }
            Err(panic) => {
                frame.destroy(device);
                std::panic::resume_unwind(panic)
            }
        }
    }

    pub fn signal(&mut self, device: &ash::Device) -> u64 {
        let value = self.count + 1;
        let signals = [vk::SemaphoreSubmitInfo::default()
            .semaphore(self.semaphore)
            .value(value)
            .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)];
        let submit = vk::SubmitInfo2::default().signal_semaphore_infos(&signals);
        if unsafe { device.queue_submit2(self.raw, &[submit], vk::Fence::null()) }.is_err() {
            return u64::MAX;
        }
        self.count = value;
        value
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
