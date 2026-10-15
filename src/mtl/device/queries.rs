use super::Device;
use crate::Error;
use crate::QueryKind;
use crate::QueryPool;
use crate::mtl::query::Query;
use objc2::rc::autoreleasepool;
use objc2_metal::{MTL4CounterHeapDescriptor, MTL4CounterHeapType, MTLDevice, MTLResourceOptions};

impl Device {
    pub fn create_query_pool(
        &self,
        kind: crate::QueryKind,
        count: u32,
    ) -> Result<QueryPool, Error> {
        autoreleasepool(|_| {
            self.caps.check_query_pool(kind, count)?;
            let heap = match kind {
                QueryKind::Timestamp => {
                    let desc = MTL4CounterHeapDescriptor::new();
                    desc.setType(MTL4CounterHeapType::Timestamp);
                    unsafe { desc.setCount(count as usize) };
                    let heap = self
                        .raw
                        .newCounterHeapWithDescriptor_error(&desc)
                        .map_err(Error::from)?;
                    Query::Timestamp(heap)
                }
                QueryKind::Occlusion | QueryKind::Precise => {
                    let options = MTLResourceOptions::StorageModePrivate
                        | MTLResourceOptions::HazardTrackingModeUntracked;
                    let buffer = self
                        .raw
                        .newBufferWithLength_options(count as usize * 8, options)
                        .ok_or_else(|| {
                            Error::OutOfMemory("visibility buffer allocation failed".into())
                        })?;
                    Query::Buffer(buffer)
                }
            };
            let id = self.res.queries.insert(heap);
            self.res.table.queries.set(id, (kind, count));
            Ok(id)
        })
    }

    pub fn destroy_query_pool(&self, heap: QueryPool) {
        self.res.table.drop_query(heap);
        self.res.queries.remove(heap);
    }
}
