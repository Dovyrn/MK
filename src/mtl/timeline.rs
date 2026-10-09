use crate::Error;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLSharedEvent;
use std::sync::{Arc, OnceLock};

pub struct Timeline {
    pub(crate) id: crate::handle::TimelineId,
    pub(crate) token: Arc<()>,
    pub(crate) raw: Retained<ProtocolObject<dyn MTLSharedEvent>>,
    pub(crate) lost: Arc<OnceLock<String>>,
}

impl Timeline {
    pub fn raw(&self) -> &ProtocolObject<dyn MTLSharedEvent> {
        &self.raw
    }

    pub fn value(&self) -> Result<u64, Error> {
        self.healthy()?;
        Ok(self.raw.signaledValue())
    }

    pub fn wait(&self, value: u64, ms: u64) -> Result<bool, Error> {
        let done = self.raw.waitUntilSignaledValue_timeoutMS(value, ms);
        self.healthy()?;
        Ok(done)
    }

    fn healthy(&self) -> Result<(), Error> {
        match self.lost.get() {
            Some(e) => Err(Error::DeviceLost(e.clone())),
            None => Ok(()),
        }
    }

    pub fn signal(&self, value: u64) -> Result<(), Error> {
        crate::table::assert_rising(self.raw.signaledValue(), value);
        self.raw.setSignaledValue(value);
        Ok(())
    }
}
