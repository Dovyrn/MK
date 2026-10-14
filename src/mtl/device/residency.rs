use super::Device;
use crate::Error;
use crate::mtl::query::Query;
use crate::residency::Members;
use crate::{Buffer, IndirectBuffer, QueryPool, ResidencySet, Resource, ResourceKind, Texture};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLAllocation, MTLDevice, MTLResidencySet, MTLResidencySetDescriptor};

impl Device {
    pub fn create_residency_set(&mut self) -> Result<ResidencySet, Error> {
        autoreleasepool(|_| {
            let raw = self
                .raw
                .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
                .map_err(Error::from)?;
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
        let res = &self.res;
        let raw = res.sets.get(set);
        for r in resources {
            let a: &ProtocolObject<dyn MTLAllocation> = match r.kind {
                ResourceKind::Buffer => {
                    ProtocolObject::from_ref(&*res.buffers.get(Buffer::from_raw(r.handle)).raw)
                }
                ResourceKind::Texture => {
                    ProtocolObject::from_ref(&*res.textures.get(Texture::from_raw(r.handle)).raw)
                }
                ResourceKind::QueryPool => match res.queries.get(QueryPool::from_raw(r.handle)) {
                    Query::Buffer(b) => ProtocolObject::from_ref(&**b),
                    Query::Timestamp(_) => continue,
                },
                ResourceKind::IndirectBuffer => ProtocolObject::from_ref(
                    &*res.indirects.get(IndirectBuffer::from_raw(r.handle)).raw,
                ),
            };
            if add {
                raw.addAllocation(a);
            } else {
                raw.removeAllocation(a);
            }
        }
        if cfg!(feature = "validation") {
            self.res
                .table
                .change_residency(set, Members::from(resources), add);
        }
    }

    pub fn commit_residency(&mut self, set: ResidencySet) {
        if cfg!(feature = "validation") {
            self.res.table.residency.commit(set);
        }
        self.res.sets.get(set).commit();
    }

    pub fn raw_residency_set(&self, set: ResidencySet) -> &ProtocolObject<dyn MTLResidencySet> {
        self.res.table.residency.assert_set(set);
        self.res.sets.get(set)
    }
}
