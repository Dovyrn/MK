use super::Device;
use crate::Error;
use crate::mtl::Timeline;
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLDevice, MTLSharedEvent};

impl Device {
    pub fn create_timeline(&self) -> Result<Timeline, Error> {
        autoreleasepool(|_| {
            let raw = self
                .raw
                .newSharedEvent()
                .ok_or_else(|| Error::OutOfMemory("timeline allocation failed".into()))?;
            Ok(self.import_timeline(raw))
        })
    }

    pub fn import_timeline(&self, raw: Retained<ProtocolObject<dyn MTLSharedEvent>>) -> Timeline {
        let id = self.res.timelines.insert(raw.clone());
        Timeline {
            id,
            token: self.res.table.add_timeline(id),
            raw,
            lost: self.lost.clone(),
        }
    }

    pub fn destroy_timeline(&mut self, timeline: Timeline) {
        self.res.table.drop_timeline(&timeline);
        let raw = self.res.timelines.remove(timeline.id);
        drop(raw);
    }
}
