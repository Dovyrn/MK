use crate::mtl::pipeline::Pipeline;
use crate::mtl::texture::Texture;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{MTLBuffer, MTLSamplerState, MTLSharedEvent};

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<Retained<ProtocolObject<dyn MTLSamplerState>>>,
    pub pipelines: Slots<Pipeline>,
    pub events: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
}
