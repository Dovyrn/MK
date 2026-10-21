use super::Device;
use crate::Error;
use crate::residency::Members;
use crate::table::lock;
use crate::{ResidencySet, ResidentResource};

impl Device {
    pub fn create_residency_set(&self) -> Result<ResidencySet, Error> {
        let id = self.res.sets.insert(());
        lock(&self.res.table.residency).create(id);
        Ok(id)
    }

    pub fn destroy_residency_set(&self, set: ResidencySet) {
        lock(&self.res.table.residency).destroy(set);
        self.res.sets.remove(set);
    }

    pub fn add_residency(&self, set: ResidencySet, resources: &[ResidentResource]) {
        if cfg!(feature = "validation") {
            self.res
                .table
                .change_residency(set, Members::from(resources), true);
        }
    }

    pub fn remove_residency(&self, set: ResidencySet, resources: &[ResidentResource]) {
        if cfg!(feature = "validation") {
            self.res
                .table
                .change_residency(set, Members::from(resources), false);
        }
    }

    pub fn commit_residency(&self, set: ResidencySet) {
        if cfg!(feature = "validation") {
            lock(&self.res.table.residency).commit(set);
        }
    }
}
