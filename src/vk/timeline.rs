use ash::vk;
use std::sync::Arc;

pub struct Timeline {
    pub(crate) id: u32,
    pub(crate) token: Arc<()>,
    pub(crate) device: ash::Device,
    pub(crate) raw: vk::Semaphore,
}

impl Timeline {
    pub fn raw(&self) -> vk::Semaphore {
        self.raw
    }

    pub fn value(&self) -> Result<u64, String> {
        unsafe { self.device.get_semaphore_counter_value(self.raw) }.map_err(|e| e.to_string())
    }

    pub fn wait(&self, value: u64, ms: u64) -> Result<bool, String> {
        let semaphores = [self.raw];
        let values = [value];
        let info = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        match unsafe {
            self.device
                .wait_semaphores(&info, ms.saturating_mul(1_000_000))
        } {
            Ok(()) => Ok(true),
            Err(vk::Result::TIMEOUT) => Ok(false),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn signal(&self, value: u64) -> Result<(), String> {
        crate::table::rising(self.value()?, value);
        let info = vk::SemaphoreSignalInfo::default()
            .semaphore(self.raw)
            .value(value);
        unsafe { self.device.signal_semaphore(&info) }.map_err(|e| e.to_string())
    }
}
