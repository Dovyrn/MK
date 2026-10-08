use super::Device;
use crate::residency::Members;
use crate::{Buffer, QueryPool, ResidencySet, Texture};

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

    pub fn add_residency(
        &mut self,
        set: ResidencySet,
        buffers: &[Buffer],
        textures: &[Texture],
        queries: &[QueryPool],
    ) {
        let items = Members {
            buffers: buffers.to_vec(),
            textures: textures.to_vec(),
            queries: queries.to_vec(),
        };
        self.res.table.change_residency(set, items, true);
    }

    pub fn remove_residency(
        &mut self,
        set: ResidencySet,
        buffers: &[Buffer],
        textures: &[Texture],
        queries: &[QueryPool],
    ) {
        let items = Members {
            buffers: buffers.to_vec(),
            textures: textures.to_vec(),
            queries: queries.to_vec(),
        };
        self.res.table.change_residency(set, items, false);
    }

    pub fn commit_residency(&mut self, set: ResidencySet) {
        self.res.table.residency.commit(set);
    }
}
