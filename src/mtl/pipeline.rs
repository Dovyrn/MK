use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLComputePipelineState, MTLSize};

pub struct Pipeline {
    pub state: Retained<ProtocolObject<dyn MTLComputePipelineState>>,
    pub group: MTLSize,
}
