use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLAllocation, MTLDevice, MTLResidencySet, MTLResidencySetDescriptor};
use std::collections::HashMap;

pub struct Residency {
    pub set: Retained<ProtocolObject<dyn MTLResidencySet>>,
    counts: HashMap<usize, u32>,
}

impl Residency {
    pub fn new(device: &ProtocolObject<dyn MTLDevice>) -> Result<Self, String> {
        Ok(Self {
            set: device
                .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
                .map_err(|e| e.localizedDescription().to_string())?,
            counts: HashMap::new(),
        })
    }

    pub fn add(&mut self, item: &ProtocolObject<dyn MTLAllocation>) {
        let count = self.counts.entry(item as *const _ as *const u8 as usize);
        let count = count.or_insert(0);
        *count += 1;
        if *count == 1 {
            self.set.addAllocation(item);
            self.set.commit();
        }
    }

    pub fn remove(&mut self, item: &ProtocolObject<dyn MTLAllocation>) {
        let key = item as *const _ as *const u8 as usize;
        let Some(count) = self.counts.get_mut(&key) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.counts.remove(&key);
            self.set.removeAllocation(item);
            self.set.commit();
        }
    }

    pub fn clear(&mut self) {
        self.counts.clear();
        self.set.removeAllAllocations();
        self.set.commit();
    }
}
