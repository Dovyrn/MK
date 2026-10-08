use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::render::Render;
use crate::mtl::texture::TextureObject;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBuffer, MTLDepthStencilState, MTLRenderPipelineState, MTLResidencySet, MTLSamplerState,
    MTLSharedEvent,
};

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pub textures: Slots<TextureObject>,
    pub samplers: Slots<Retained<ProtocolObject<dyn MTLSamplerState>>>,
    pub pipelines: Slots<Pipeline>,
    pub renders: Slots<Render>,
    pub tiles: Slots<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
    pub depths: Slots<Retained<ProtocolObject<dyn MTLDepthStencilState>>>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
    pub indirects: Slots<crate::mtl::indirect::Indirect>,
    pub sets: Slots<Retained<ProtocolObject<dyn MTLResidencySet>>>,
    pub table: crate::table::Table,
}
