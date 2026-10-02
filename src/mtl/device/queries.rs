use super::Device;
use crate::mtl::query::Query;
use crate::query;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4CounterHeapDescriptor, MTL4CounterHeapType, MTLDevice, MTLResourceOptions};

impl Device {
    pub fn create_query_heap(&mut self, kind: u32, count: u32) -> Result<u32, String> {
        let heap = match kind {
            query::TIMESTAMP => {
                let desc = MTL4CounterHeapDescriptor::new();
                desc.setType(MTL4CounterHeapType::Timestamp);
                unsafe { desc.setCount(count as usize) };
                let heap = self
                    .raw
                    .newCounterHeapWithDescriptor_error(&desc)
                    .map_err(|e| e.localizedDescription().to_string())?;
                Query::Timestamp(heap)
            }
            query::STATISTICS => return Err("pipeline statistics are not supported".into()),
            query::OCCLUSION | query::PRECISE | query::COMPACTED => {
                let options = MTLResourceOptions::StorageModePrivate
                    | MTLResourceOptions::HazardTrackingModeUntracked;
                let buffer = self
                    .raw
                    .newBufferWithLength_options(count as usize * 8, options)
                    .ok_or("visibility buffer")?;
                self.residency.add(ProtocolObject::from_ref(&*buffer));
                Query::Buffer(buffer)
            }
            k => panic!("unknown query kind {k}"),
        };
        let id = self.res.queries.insert(heap);
        self.res.table.heaps.set(id, (kind, count));
        Ok(id)
    }

    pub fn destroy_query_heap(&mut self, heap: u32) {
        self.res.table.heaps.clear(heap);
        let heap = self.res.queries.remove(heap);
        self.discard(move |r| {
            if let Query::Buffer(buffer) = &heap {
                r.remove(ProtocolObject::from_ref(&**buffer));
            }
        });
    }
}
