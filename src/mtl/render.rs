use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDepthClipMode, MTLPrimitiveType, MTLRenderPipelineState, MTLSize, MTLTriangleFillMode,
};

pub struct Render {
    pub state: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub fill: MTLTriangleFillMode,
    pub clip: MTLDepthClipMode,
    pub primitive: MTLPrimitiveType,
    pub task: MTLSize,
    pub mesh: MTLSize,
}
