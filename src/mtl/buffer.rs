use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLBuffer;

pub struct BufferObject {
    pub raw: Retained<ProtocolObject<dyn MTLBuffer>>,
    pub address: u64,
    pub length: usize,
}

impl BufferObject {
    pub fn new(raw: Retained<ProtocolObject<dyn MTLBuffer>>) -> Self {
        Self {
            address: raw.gpuAddress(),
            length: raw.length(),
            raw,
        }
    }
}
