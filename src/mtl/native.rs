use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTL4ArgumentTable, MTL4CommandBuffer, MTL4ComputeCommandEncoder, MTL4RenderCommandEncoder,
};

pub struct Native<'a> {
    pub buffer: &'a ProtocolObject<dyn MTL4CommandBuffer>,
    pub table: &'a ProtocolObject<dyn MTL4ArgumentTable>,
    pub compute: Option<&'a ProtocolObject<dyn MTL4ComputeCommandEncoder>>,
    pub render: Option<&'a ProtocolObject<dyn MTL4RenderCommandEncoder>>,
}

pub type Callback = Box<dyn Fn(&Native)>;
