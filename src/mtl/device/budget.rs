use super::Device;
use crate::MemoryBudget;
use objc2_metal::MTLDevice;

impl Device {
    pub fn memory_budget(&self) -> Result<MemoryBudget, String> {
        Ok(MemoryBudget {
            budget: self.raw.recommendedMaxWorkingSetSize(),
            usage: self.raw.currentAllocatedSize() as u64,
        })
    }
}
