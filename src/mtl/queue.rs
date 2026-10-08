use crate::Error;
use crate::handle::TimelineId;
use crate::mtl::resources::Resources;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandBuffer, MTL4CommandQueue, MTL4CommitFeedback, MTL4CommitOptions, MTLDevice, MTLEvent,
};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

pub struct Queue {
    pub raw: Retained<ProtocolObject<dyn MTL4CommandQueue>>,
    lost: Arc<Mutex<Option<String>>>,
}

impl Queue {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        lost: Arc<Mutex<Option<String>>>,
    ) -> Result<Self, Error> {
        let raw = device
            .newMTL4CommandQueue()
            .ok_or_else(|| Error::OutOfMemory("command queue allocation failed".into()))?;
        Ok(Self { raw, lost })
    }

    pub fn lost(&self) -> Result<(), Error> {
        match self.lost.lock().unwrap().clone() {
            Some(e) => Err(Error::DeviceLost(e)),
            None => Ok(()),
        }
    }

    pub fn commit(
        &self,
        buffer: &ProtocolObject<dyn MTL4CommandBuffer>,
        res: &Resources,
        waits: &[(TimelineId, u64)],
        signals: &[(TimelineId, u64)],
    ) {
        for &(event, value) in waits {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**res.timelines.get(event));
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
        let mut buffer = NonNull::from(buffer);
        unsafe {
            self.raw
                .commit_count_options(NonNull::from(&mut buffer), 1, &options)
        };
        for &(event, value) in signals {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&**res.timelines.get(event));
            self.raw.signalEvent_value(event, value);
        }
    }
}
