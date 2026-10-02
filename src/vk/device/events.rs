use super::Device;
use crate::vk::queue::timeline;
use ash::vk;

impl Device {
    pub fn raw_event(&self, event: u32) -> vk::Semaphore {
        self.res.events.get(event).0
    }

    pub fn create_event(&mut self) -> Result<u32, String> {
        let event = timeline(&self.raw).map_err(|e| e.to_string())?;
        Ok(self.res.events.insert((event, true)))
    }

    pub fn import_event(&mut self, raw: vk::Semaphore) -> u32 {
        self.res.events.insert((raw, false))
    }

    pub fn destroy_event(&mut self, event: u32) {
        self.defer(move |d| {
            let (event, owned) = d.res.events.remove(event);
            if owned {
                unsafe { d.raw.destroy_semaphore(event, None) };
            }
        });
    }

    pub fn event_value(&mut self, event: u32) -> Result<u64, String> {
        self.collect();
        unsafe {
            self.raw
                .get_semaphore_counter_value(self.res.events.get(event).0)
                .map_err(|e| e.to_string())
        }
    }

    pub fn wait(&mut self, event: u32, value: u64, ms: u64) -> Result<bool, String> {
        self.collect();
        let semaphores = [self.res.events.get(event).0];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        unsafe {
            match self
                .raw
                .wait_semaphores(&info, ms.saturating_mul(1_000_000))
            {
                Ok(()) => Ok(true),
                Err(vk::Result::TIMEOUT) => Ok(false),
                Err(e) => Err(e.to_string()),
            }
        }
    }
}
