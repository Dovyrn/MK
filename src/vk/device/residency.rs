use super::Device;
use crate::Error;
use crate::residency::Members;
use crate::{ResidencySet, Resource};

impl Device {
    pub fn create_residency_set(&mut self) -> Result<ResidencySet, Error> {
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
        self.res.table.residency.commit(set);
    }
}
