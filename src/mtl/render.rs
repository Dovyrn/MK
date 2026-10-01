use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLRenderPipelineState, MTLTriangleFillMode};

pub struct Render {
    pub state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub fill: MTLTriangleFillMode,
}
