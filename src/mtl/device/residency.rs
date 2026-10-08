use super::Device;
use crate::mtl::query::Query;
use crate::residency::Members;
use crate::{ResidencySet, Resource};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLAllocation, MTLDevice, MTLResidencySet, MTLResidencySetDescriptor};

impl Device {
    pub fn create_residency_set(&mut self) -> Result<ResidencySet, String> {
        autoreleasepool(|_| {
            let raw = self
                .raw
                .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
                .map_err(|e| e.localizedDescription().to_string())?;
            let id = self.res.sets.insert(raw);
            self.res.table.residency.create(id);
            Ok(id)
        })
    }

    pub fn destroy_residency_set(&mut self, set: ResidencySet) {
        self.res.table.residency.destroy(set);
        self.res.sets.remove(set);
    }

    pub fn add_residency(&mut self, set: ResidencySet, resources: &[Resource]) {
        self.change_residency(set, resources, true);
    }

    pub fn remove_residency(&mut self, set: ResidencySet, resources: &[Resource]) {
        self.change_residency(set, resources, false);
    }

    fn change_residency(&mut self, set: ResidencySet, resources: &[Resource], add: bool) {
        let items = Members::from(resources);
        let res = &self.res;
        let allocations: Vec<&ProtocolObject<dyn MTLAllocation>> = items
            .buffers
            .iter()
            .map(|&b| ProtocolObject::from_ref(&**res.buffers.get(b)))
            .chain(
                items
                    .textures
                    .iter()
                    .map(|&t| ProtocolObject::from_ref(&*res.textures.get(t).raw)),
            )
            .chain(
                items
                    .queries
                    .iter()
                    .filter_map(|&q| match res.queries.get(q) {
                        Query::Buffer(b) => Some(ProtocolObject::from_ref(&**b)),
                        Query::Timestamp(_) => None,
                    }),
            )
            .chain(
                items
                    .indirects
                    .iter()
                    .map(|&i| ProtocolObject::from_ref(&*res.indirects.get(i).raw)),
            )
            .collect();
        let raw = res.sets.get(set);
        for a in allocations {
            if add {
                raw.addAllocation(a);
            } else {
                raw.removeAllocation(a);
            }
        }
        self.res.table.change_residency(set, items, add);
    }

    pub fn commit_residency(&mut self, set: ResidencySet) {
        self.res.table.residency.commit(set);
        self.res.sets.get(set).commit();
    }

    pub fn raw_residency_set(&self, set: ResidencySet) -> &ProtocolObject<dyn MTLResidencySet> {
        self.res.table.residency.assert_set(set);
        self.res.sets.get(set)
    }
}
