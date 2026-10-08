use super::Device;
use crate::Error;
use crate::MemoryBudget;
use objc2_metal::MTLDevice;

impl Device {
    pub fn memory_budget(&self) -> Result<MemoryBudget, Error> {
        Ok(MemoryBudget {
            budget: self.raw.recommendedMaxWorkingSetSize(),
            usage: self.raw.currentAllocatedSize() as u64,
        })
    }
}
