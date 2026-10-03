use super::Device;
use crate::Budget;
use ash::vk;

impl Device {
    pub fn memory_budget(&self) -> Result<Budget, String> {
        if !self.ext.budget {
            return Err("memory budgets need VK_EXT_memory_budget".into());
        }
        let mut heaps = vk::PhysicalDeviceMemoryBudgetPropertiesEXT::default();
        let mut properties = vk::PhysicalDeviceMemoryProperties2::default().push_next(&mut heaps);
        unsafe {
            self.instance
                .get_physical_device_memory_properties2(self.physical, &mut properties)
        };
        let count = self.memory.memory_heap_count as usize;
        let local = (0..count).filter(|&i| {
            self.memory.memory_heaps[i]
                .flags
                .contains(vk::MemoryHeapFlags::DEVICE_LOCAL)
        });
        Ok(local.fold(Budget::default(), |b, i| Budget {
            budget: b.budget + heaps.heap_budget[i],
            usage: b.usage + heaps.heap_usage[i],
        }))
    }
}
