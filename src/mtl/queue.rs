use crate::mtl::Callback;
use crate::mtl::count::Count;
use crate::mtl::frame::Frame;
use crate::mtl::recorder::{Recorder, VERTICES};
use crate::mtl::resolve::Resolve;
use crate::mtl::resources::Resources;
use crate::register;
use crate::slot;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTL4CommandBuffer, MTL4CommandEncoder,
    MTL4CommandQueue, MTLDevice, MTLEvent, MTLResidencySet, MTLResidencySetDescriptor,
    MTLSharedEvent,
};
use objc2_metal::{MTL4CommitFeedback, MTL4CommitOptions};
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

pub struct Queue {
    pub raw: Retained<ProtocolObject<dyn MTL4CommandQueue>>,
    pub buffer: Retained<ProtocolObject<dyn MTL4CommandBuffer>>,
    pub table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
    pub ml: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
    pub residency: Retained<ProtocolObject<dyn MTLResidencySet>>,
    event: Retained<ProtocolObject<dyn MTLSharedEvent>>,
    pub count: u64,
    free: Vec<Frame>,
    busy: VecDeque<(Frame, u64)>,
    lost: Arc<Mutex<Option<String>>>,
}

impl Queue {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        shared: &ProtocolObject<dyn MTLResidencySet>,
        lost: Arc<Mutex<Option<String>>>,
    ) -> Result<Self, String> {
        let raw = device
            .newMTL4CommandQueue()
            .ok_or("command queue allocation failed")?;
        let residency = device
            .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
            .map_err(|e| e.localizedDescription().to_string())?;
        raw.addResidencySet(shared);
        raw.addResidencySet(&residency);
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(VERTICES + slot::VERTICES as usize);
        desc.setMaxTextureBindCount((register::IMAGE + slot::IMAGES) as usize);
        desc.setMaxSamplerStateBindCount(slot::SAMPLERS as usize);
        let table = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(slot::STORAGES as usize);
        let ml = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(Self {
            raw,
            buffer: device
                .newCommandBuffer()
                .ok_or("command buffer allocation failed")?,
            table,
            ml,
            residency,
            event: device.newSharedEvent().ok_or("event allocation failed")?,
            count: 0,
            free: Vec::new(),
            busy: VecDeque::new(),
            lost,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn submit(
        &mut self,
        device: &ProtocolObject<dyn MTLDevice>,
        res: &Resources,
        resolve: &Resolve,
        count: &Count,
        (commands, natives): (&[u8], &[Callback]),
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), String> {
        if let Some(e) = self.lost.lock().unwrap().clone() {
            return Err(e);
        }
        let done = self.event.signaledValue();
        while let Some((mut frame, _)) = self.busy.pop_front_if(|(_, v)| *v <= done) {
            frame.reset(&self.residency);
            self.free.push(frame);
        }
        let mut frame = match self.free.pop() {
            Some(f) => f,
            None => Frame::new(device)?,
        };
        self.buffer
            .beginCommandBufferWithAllocator(&frame.allocator);
        let mut recorder = Recorder::new(device, self, resolve, count, &mut frame, res, natives);
        let result = catch_unwind(AssertUnwindSafe(|| {
            recorder.run(commands)?;
            recorder.release()
        }));
        if let Some(e) = recorder.compute.take() {
            e.endEncoding();
        }
        if let Some(e) = recorder.render.take() {
            e.endEncoding();
        }
        self.buffer.endCommandBuffer();
        if !matches!(result, Ok(Ok(()))) {
            frame.reset(&self.residency);
            self.free.push(frame);
            return result
                .unwrap_or_else(|p| resume_unwind(p))
                .map_err(String::from);
        }
        for &(event, value) in waits {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**res.events.get(event));
            self.raw.waitForEvent_value(event, value);
        }
        let lost = self.lost.clone();
        let handler = RcBlock::new(
            move |feedback: NonNull<ProtocolObject<dyn MTL4CommitFeedback>>| {
                if let Some(e) = unsafe { feedback.as_ref() }.error() {
                    let mut slot = lost.lock().unwrap();
                    slot.get_or_insert_with(|| e.localizedDescription().to_string());
                }
            },
        );
        let options = MTL4CommitOptions::new();
        unsafe { options.addFeedbackHandler(RcBlock::as_ptr(&handler)) };
        let mut buffer = NonNull::from(&*self.buffer);
        unsafe {
            self.raw
                .commit_count_options(NonNull::from(&mut buffer), 1, &options)
        };
        for &(event, value) in signals {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**res.events.get(event));
            self.raw.signalEvent_value(event, value);
        }
        self.signal();
        self.busy.push_back((frame, self.count));
        Ok(())
    }

    pub fn signal(&mut self) {
        self.count += 1;
        let own = ProtocolObject::<dyn MTLEvent>::from_ref(&*self.event);
        self.raw.signalEvent_value(own, self.count);
    }

    pub fn done(&self) -> u64 {
        self.event.signaledValue()
    }

    pub fn idle(&self) {
        self.event
            .waitUntilSignaledValue_timeoutMS(self.count, u64::MAX);
    }
}
