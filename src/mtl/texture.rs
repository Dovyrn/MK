use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLResourceID, MTLTexture};

pub struct TextureObject {
    pub raw: Retained<ProtocolObject<dyn MTLTexture>>,
    pub format: crate::Format,
    pub id: MTLResourceID,
}
