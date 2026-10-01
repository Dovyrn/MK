use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::MTLTexture;

pub struct Texture {
    pub raw: Retained<ProtocolObject<dyn MTLTexture>>,
    pub format: u32,
}
