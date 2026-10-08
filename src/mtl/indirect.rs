use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLIndirectCommandBuffer;

pub struct Indirect {
    pub raw: Retained<ProtocolObject<dyn MTLIndirectCommandBuffer>>,
    pub groups: [u32; 6],
}
