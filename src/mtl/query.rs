use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4CounterHeap, MTLBuffer};

pub enum Query {
    Timestamp(Retained<ProtocolObject<dyn MTL4CounterHeap>>),
    Buffer(Retained<ProtocolObject<dyn MTLBuffer>>),
}
