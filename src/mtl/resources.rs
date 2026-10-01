use crate::mtl::accel::Accel;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::render::Render;
use crate::mtl::surface::Surface;
use crate::mtl::table::Table;
use crate::mtl::texture::Texture;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBuffer, MTLDepthStencilState, MTLHeap, MTLIndirectCommandBuffer, MTLSamplerState,
    MTLSharedEvent,
};

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<Retained<ProtocolObject<dyn MTLSamplerState>>>,
    pub pipelines: Slots<Pipeline>,
    pub renders: Slots<Render>,
    pub depths: Slots<Retained<ProtocolObject<dyn MTLDepthStencilState>>>,
    pub heaps: Slots<(Retained<ProtocolObject<dyn MTLHeap>>, u32)>,
    pub indirects: Slots<Retained<ProtocolObject<dyn MTLIndirectCommandBuffer>>>,
    pub tables: Slots<Table>,
    pub accels: Slots<Accel>,
    pub queries: Slots<Query>,
    pub surfaces: Slots<Surface>,
    pub events: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
}
