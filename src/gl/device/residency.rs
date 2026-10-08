use super::Device;
use crate::residency::Members;
use crate::{ResidencySet, Resource};

impl Device {
    pub fn create_residency_set(&mut self) -> Result<ResidencySet, String> {
        let id = self.res.sets.insert(());
        self.res.table.residency.create(id);
        Ok(id)
    }

    pub fn destroy_residency_set(&mut self, set: ResidencySet) {
        self.res.table.residency.destroy(set);
        self.res.sets.remove(set);
    }

    pub fn add_residency(&mut self, set: ResidencySet, resources: &[Resource]) {
        self.res
            .table
            .change_residency(set, Members::from(resources), true);
    }

    pub fn remove_residency(&mut self, set: ResidencySet, resources: &[Resource]) {
        self.res
            .table
            .change_residency(set, Members::from(resources), false);
    }

    pub fn commit_residency(&mut self, set: ResidencySet) {
        self.thread();
        let (entered, left) = self.res.table.residency.commit(set);
        let Some(b) = self.extra.bindless.as_ref() else {
            return;
        };
        for (list, on) in [(entered, true), (left, false)] {
            let views = self
                .res
                .table
                .parents
                .iter()
                .filter(|(_, p)| list.contains(p))
                .map(|(v, _)| v);
            let all: Vec<_> = list.iter().copied().chain(views).collect();
            for t in all {
                let handle = self.res.textures.get(t).image.get();
                if handle == 0 {
                    continue;
                }
                unsafe {
                    if on {
                        (b.resident)(handle, glow::READ_WRITE);
                    } else {
                        (b.nonresident)(handle);
                    }
                }
            }
        }
    }
}
