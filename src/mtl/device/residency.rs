use super::Device;
use crate::Error;
use crate::mtl::query::Query;
use crate::residency::Members;
use crate::table::lock;
use crate::{
    Buffer, IndirectBuffer, QueryPool, ResidencySet, ResidentResource, ResourceKind, Texture,
};
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4CommandQueue, MTLAllocation, MTLDevice, MTLResidencySet, MTLResidencySetDescriptor,
};

impl Device {
    pub fn create_residency_set(&self) -> Result<ResidencySet, Error> {
        autoreleasepool(|_| {
            let raw = self
                .raw
                .newResidencySetWithDescriptor_error(&MTLResidencySetDescriptor::new())
                .map_err(Error::from)?;
            let id = self.res.sets.insert(raw);
            lock(&self.res.table.residency).create(id);
            Ok(id)
        })
    }

    pub fn destroy_residency_set(&self, set: ResidencySet) {
        lock(&self.res.table.residency).destroy(set);
        self.res.sets.remove(set);
    }

    pub fn add_residency(&self, set: ResidencySet, resources: &[ResidentResource]) {
        self.change_residency(set, resources, true);
    }

    pub fn remove_residency(&self, set: ResidencySet, resources: &[ResidentResource]) {
        self.change_residency(set, resources, false);
    }

    fn change_residency(&self, set: ResidencySet, resources: &[ResidentResource], add: bool) {
        let _held = lock(&self.residency);
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

    pub fn commit_residency(&self, set: ResidencySet) {
        let _held = lock(&self.residency);
        if cfg!(feature = "validation") {
            lock(&self.res.table.residency).commit(set);
        }
        self.res.sets.get(set).commit();
    }

    pub fn attach_residency_set(&self, queue: u32, set: ResidencySet) {
        let queue = queue as usize;
        self.check_queue(queue);
        if cfg!(feature = "validation") {
            lock(&self.res.table.residency).attach(queue, set);
        }
        self.queues[queue]
            .raw
            .addResidencySet(self.res.sets.get(set));
    }

    pub fn detach_residency_set(&self, queue: u32, set: ResidencySet) {
        let queue = queue as usize;
        self.check_queue(queue);
        if cfg!(feature = "validation") {
            lock(&self.res.table.residency).detach(queue, set);
        }
        self.queues[queue]
            .raw
            .removeResidencySet(self.res.sets.get(set));
    }

    pub fn raw_residency_set(&self, set: ResidencySet) -> &ProtocolObject<dyn MTLResidencySet> {
        lock(&self.res.table.residency).assert_set(set);
        self.res.sets.get(set)
    }
}
