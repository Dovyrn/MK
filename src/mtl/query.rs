use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4CounterHeap, MTLBuffer};

pub enum Query {
    Timestamp(Retained<ProtocolObject<dyn MTL4CounterHeap>>),
    Buffer(Retained<ProtocolObject<dyn MTLBuffer>>),
}

impl Query {
    pub fn check(&self, first: u32, count: u32) {
        let total = match self {
            Query::Timestamp(heap) => heap.count(),
            Query::Buffer(buffer) => buffer.length() / 8,
        };
        if count > 0 {
            let last = first as usize + count as usize - 1;
            assert!(last < total, "query {last} is outside 0..{total}");
        }
    }
}
