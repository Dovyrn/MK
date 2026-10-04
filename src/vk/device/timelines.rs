use super::Device;
use crate::vk::Timeline;
use crate::vk::queue::timeline;
use crate::vk::semaphore::Semaphore;
use ash::vk;

impl Device {
    pub fn create_timeline(&mut self) -> Result<Timeline, String> {
        let raw = timeline(&self.raw).map_err(|e| e.to_string())?;
        Ok(self.add_timeline(raw, true))
    }

    pub fn import_timeline(&mut self, raw: vk::Semaphore) -> Timeline {
        self.add_timeline(raw, false)
    }

    pub fn destroy_timeline(&mut self, timeline: Timeline) {
        self.res.table.drop_timeline(&timeline);
        let id = timeline.id;
        self.defer(move |d| {
            d.res.timelines.remove(id).free(&d.raw);
        });
    }

    fn add_timeline(&mut self, raw: vk::Semaphore, own: bool) -> Timeline {
        let id = self.res.timelines.insert(Semaphore { raw, own });
        Timeline {
            id,
            token: self.res.table.add_timeline(id),
            device: self.raw.clone(),
            raw,
        }
    }
}
