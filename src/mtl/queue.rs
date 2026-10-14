use crate::{Error, Timeline};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandBuffer, MTL4CommandQueue, MTL4CommitFeedback, MTL4CommitOptions, MTLDevice, MTLEvent,
};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

pub struct Queue {
    pub raw: Retained<ProtocolObject<dyn MTL4CommandQueue>>,
    options: Retained<MTL4CommitOptions>,
    lost: Arc<OnceLock<String>>,
    pub scratch: Mutex<Vec<Retained<ProtocolObject<dyn MTL4CommandBuffer>>>>,
}

impl Queue {
    pub fn new(
        device: &ProtocolObject<dyn MTLDevice>,
        lost: Arc<OnceLock<String>>,
    ) -> Result<Self, Error> {
        let raw = device
            .newMTL4CommandQueue()
            .ok_or_else(|| Error::OutOfMemory("command queue allocation failed".into()))?;
        let sink = lost.clone();
        let handler = RcBlock::new(
            move |feedback: NonNull<ProtocolObject<dyn MTL4CommitFeedback>>| {
                if let Some(e) = unsafe { feedback.as_ref() }.error() {
                    let _ = sink.set(e.localizedDescription().to_string());
                }
            },
        );
        let options = MTL4CommitOptions::new();
        unsafe { options.addFeedbackHandler(RcBlock::as_ptr(&handler)) };
        Ok(Self {
            raw,
            options,
            lost,
            scratch: Mutex::default(),
        })
    }

    pub fn lost(&self) -> Result<(), Error> {
        match self.lost.get() {
            Some(e) => Err(Error::DeviceLost(e.clone())),
            None => Ok(()),
        }
    }

    pub fn commit(
        &self,
        buffers: &[Retained<ProtocolObject<dyn MTL4CommandBuffer>>],
        waits: &[(&Timeline, u64)],
        signals: &[(&Timeline, u64)],
    ) {
        for &(t, value) in waits {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&*t.raw);
            self.raw.waitForEvent_value(event, value);
        }
        let list = NonNull::from(buffers).cast();
        unsafe {
            self.raw
                .commit_count_options(list, buffers.len(), &self.options)
        };
        for &(t, value) in signals {
            let event = ProtocolObject::<dyn MTLEvent>::from_ref(&*t.raw);
            self.raw.signalEvent_value(event, value);
        }
    }
}
