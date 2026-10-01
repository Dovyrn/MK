use crate::mtl::frame::Frame;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::recorder::Recorder;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4ArgumentTable, MTL4ArgumentTableDescriptor, MTL4CommandBuffer, MTL4CommandEncoder,
    MTL4CommandQueue, MTLBuffer, MTLDevice, MTLEvent, MTLResidencySet, MTLResidencySetDescriptor,
    MTLSharedEvent, MTLSize,
};
use std::collections::VecDeque;
use std::ptr::NonNull;

pub struct Queue {
    raw: Retained<ProtocolObject<dyn MTL4CommandQueue>>,
    buffer: Retained<ProtocolObject<dyn MTL4CommandBuffer>>,
    table: Retained<ProtocolObject<dyn MTL4ArgumentTable>>,
    residency: Retained<ProtocolObject<dyn MTLResidencySet>>,
    event: Retained<ProtocolObject<dyn MTLSharedEvent>>,
    count: u64,
    free: Vec<Frame>,
    busy: VecDeque<(Frame, u64)>,
}

impl Queue {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        shared: &ProtocolObject<dyn MTLResidencySet>,
    ) -> Result<Self, String> {
        let raw = device
            .newMTL4CommandQueue()
            .ok_or("no MTL4 command queue")?;
        let residency = device
            .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
            .map_err(|e| e.localizedDescription().to_string())?;
        raw.addResidencySet(shared);
        raw.addResidencySet(&residency);
        let desc = MTL4ArgumentTableDescriptor::new();
        desc.setMaxBufferBindCount(1);
        let table = device
            .newArgumentTableWithDescriptor_error(&desc)
            .map_err(|e| e.localizedDescription().to_string())?;
        Ok(Self {
            raw,
            buffer: device.newCommandBuffer().ok_or("no MTL4 command buffer")?,
            table,
            residency,
            event: device.newSharedEvent().ok_or("no shared event")?,
            count: 0,
            free: Vec::new(),
            busy: VecDeque::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn submit(
        &mut self,
        device: &ProtocolObject<dyn MTLDevice>,
        buffers: &Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
        pipelines: &Slots<Pipeline>,
        events: &Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
        commands: &[u8],
        waits: &[(u32, u64)],
        signals: &[(u32, u64)],
    ) -> Result<(), &'static str> {
        let done = self.event.signaledValue();
        while self.busy.front().is_some_and(|(_, v)| *v <= done) {
            let (mut frame, _) = self.busy.pop_front().unwrap();
            frame.reset();
            self.free.push(frame);
        }
        let mut frame = self.free.pop().unwrap_or_else(|| Frame::new(device));
        self.buffer
            .beginCommandBufferWithAllocator(&frame.allocator);
        let mut recorder = Recorder {
            device,
            residency: &self.residency,
            buffer: &self.buffer,
            table: &self.table,
            frame: &mut frame,
            buffers,
            pipelines,
            encoder: None,
            group: MTLSize {
                width: 1,
                height: 1,
                depth: 1,
            },
            push: [0; 256],
            size: 0,
            dirty: false,
        };
        let result = recorder.run(commands);
        if let Some(e) = recorder.encoder.take() {
            e.endEncoding();
        }
        self.buffer.endCommandBuffer();
        if let Err(name) = result {
            frame.reset();
            self.free.push(frame);
            return Err(name);
        }
        for &(event, value) in waits {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**events.get(event));
            self.raw.waitForEvent_value(event, value);
        }
        let mut buffer = NonNull::from(&*self.buffer);
        unsafe { self.raw.commit_count(NonNull::from(&mut buffer), 1) };
        for &(event, value) in signals {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**events.get(event));
            self.raw.signalEvent_value(event, value);
        }
        self.count += 1;
        let own = ProtocolObject::<dyn MTLEvent>::from_ref(&*self.event);
        self.raw.signalEvent_value(own, self.count);
        self.busy.push_back((frame, self.count));
        Ok(())
    }
}
