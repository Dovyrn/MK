use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTL4AccelerationStructureDescriptor, MTLAccelerationStructure};

pub struct Accel {
    pub raw: Retained<ProtocolObject<dyn MTLAccelerationStructure>>,
    pub desc: Retained<MTL4AccelerationStructureDescriptor>,
    pub build: u64,
    pub update: u64,
}
