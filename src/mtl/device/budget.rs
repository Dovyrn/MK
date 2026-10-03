use super::Device;
use crate::Budget;
use objc2_metal::MTLDevice;

impl Device {
    pub fn memory_budget(&self) -> Result<Budget, String> {
        Ok(Budget {
            budget: self.raw.recommendedMaxWorkingSetSize(),
            usage: self.raw.currentAllocatedSize() as u64,
        })
    }
}
