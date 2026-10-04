use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLSharedEvent;
use std::sync::{Arc, Mutex};

pub struct Timeline {
    pub(crate) id: u32,
    pub(crate) token: Arc<()>,
    pub(crate) raw: Retained<ProtocolObject<dyn MTLSharedEvent>>,
    pub(crate) lost: Arc<Mutex<Option<String>>>,
}

impl Timeline {
    pub fn raw(&self) -> &ProtocolObject<dyn MTLSharedEvent> {
        &self.raw
    }

    pub fn value(&self) -> Result<u64, String> {
        self.healthy()?;
        Ok(self.raw.signaledValue())
    }

    pub fn wait(&self, value: u64, ms: u64) -> Result<bool, String> {
        let done = self.raw.waitUntilSignaledValue_timeoutMS(value, ms);
        self.healthy()?;
        Ok(done)
    }

    fn healthy(&self) -> Result<(), String> {
        match self.lost.lock().unwrap().clone() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    pub fn signal(&self, value: u64) -> Result<(), String> {
        crate::table::rising(self.raw.signaledValue(), value);
        self.raw.setSignaledValue(value);
        Ok(())
    }
}
