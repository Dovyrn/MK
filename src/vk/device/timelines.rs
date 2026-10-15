use super::Device;
use crate::Error;
use crate::vk::Timeline;
use crate::vk::queue::timeline;
use crate::vk::semaphore::Semaphore;
use ash::vk;

impl Device {
    pub fn create_timeline(&self) -> Result<Timeline, Error> {
        let raw = timeline(&self.raw).map_err(Error::from)?;
        Ok(self.add_timeline(raw, true))
    }

    pub fn import_timeline(&self, raw: vk::Semaphore) -> Timeline {
        self.add_timeline(raw, false)
    }

    pub fn destroy_timeline(&self, timeline: Timeline) {
        self.res.table.drop_timeline(&timeline);
        let id = timeline.id;
        self.res.timelines.remove(id).free(&self.raw);
    }

    pub(super) fn add_timeline(&self, raw: vk::Semaphore, own: bool) -> Timeline {
        let id = self.res.timelines.insert(Semaphore { raw, own });
        Timeline {
            id,
            token: self.res.table.add_timeline(id),
            device: self.raw.clone(),
            raw,
        }
    }
}
