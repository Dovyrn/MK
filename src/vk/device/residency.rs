use super::Device;
use crate::Error;
use crate::residency::Members;
use crate::{ResidencySet, Resource};

impl Device {
    pub fn create_residency_set(&self) -> Result<ResidencySet, Error> {
        let id = self.res.sets.insert(());
        self.res.table.residency.borrow_mut().create(id);
        Ok(id)
    }

    pub fn destroy_residency_set(&mut self, set: ResidencySet) {
        self.res.table.residency.get_mut().destroy(set);
        self.res.sets.remove(set);
    }

    pub fn add_residency(&self, set: ResidencySet, resources: &[Resource]) {
        if cfg!(feature = "validation") {
            self.res
                .table
                .change_residency(set, Members::from(resources), true);
        }
    }

    pub fn remove_residency(&self, set: ResidencySet, resources: &[Resource]) {
        if cfg!(feature = "validation") {
            self.res
                .table
                .change_residency(set, Members::from(resources), false);
        }
    }

    pub fn commit_residency(&self, set: ResidencySet) {
        if cfg!(feature = "validation") {
            self.res.table.residency.borrow_mut().commit(set);
        }
    }
}
