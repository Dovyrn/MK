use super::Device;
use crate::QueryPool;
use crate::mtl::query::Query;
use crate::query_kind;
use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4CounterHeapDescriptor, MTL4CounterHeapType, MTLDevice, MTLResourceOptions};

impl Device {
    pub fn create_query_pool(&mut self, kind: u32, count: u32) -> Result<QueryPool, String> {
        autoreleasepool(|_| {
            self.caps.check_query_pool(kind, count)?;
            let heap = match kind {
                query_kind::TIMESTAMP => {
                    let desc = MTL4CounterHeapDescriptor::new();
                    desc.setType(MTL4CounterHeapType::Timestamp);
                    unsafe { desc.setCount(count as usize) };
                    let heap = self
                        .raw
                        .newCounterHeapWithDescriptor_error(&desc)
                        .map_err(|e| e.localizedDescription().to_string())?;
                    Query::Timestamp(heap)
                }
                query_kind::OCCLUSION | query_kind::PRECISE => {
                    let options = MTLResourceOptions::StorageModePrivate
                        | MTLResourceOptions::HazardTrackingModeUntracked;
                    let buffer = self
                        .raw
                        .newBufferWithLength_options(count as usize * 8, options)
                        .ok_or("visibility buffer allocation failed")?;
                    self.residency.add(ProtocolObject::from_ref(&*buffer));
                    Query::Buffer(buffer)
                }
                k => panic!("unknown query kind {k}"),
            };
            let id = self.res.queries.insert(heap);
            self.res.table.queries.set(id, (kind, count));
            Ok(id)
        })
    }

    pub fn destroy_query_pool(&mut self, heap: QueryPool) {
        self.res.table.queries.clear(heap);
        let heap = self.res.queries.remove(heap);
        if let Query::Buffer(buffer) = &heap {
            self.residency.remove(ProtocolObject::from_ref(&**buffer));
        }
    }
}
