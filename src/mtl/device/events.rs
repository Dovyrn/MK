use super::Device;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLSharedEvent};

impl Device {
    pub fn import_event(&mut self, raw: Retained<ProtocolObject<dyn MTLSharedEvent>>) -> u32 {
        let id = self.res.events.insert(raw);
        self.res.table.events.set(id, (0, usize::MAX));
        id
    }

    pub fn raw_event(&self, event: u32) -> &ProtocolObject<dyn MTLSharedEvent> {
        self.res.events.get(event)
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        let event = self.raw.newSharedEvent().ok_or("event allocation failed")?;
        let id = self.res.events.insert(event);
        self.res.table.events.set(id, (0, usize::MAX));
        Ok(id)
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.res.table.events.clear(event);
        let event = self.res.events.remove(event);
        self.defer(move |_| drop(event));
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.collect();
        self.healthy()?;
        Ok(self.res.events.get(event).signaledValue())
    }

    pub fn wait(&mut self, event: u32, value: u64, ms: u64) -> Result<bool, String> {
        let done = self
            .res
            .events
            .get(event)
            .waitUntilSignaledValue_timeoutMS(value, ms);
        self.collect();
        self.healthy()?;
        Ok(done)
    }
}
