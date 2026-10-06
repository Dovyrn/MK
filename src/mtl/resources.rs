use crate::mtl::accel::Accel;
use crate::mtl::indirect::Indirect;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::render::Render;
use crate::mtl::texture::Texture;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLBuffer, MTLDepthStencilState, MTLHeap, MTLRenderPipelineState, MTLSamplerState,
    MTLSharedEvent,
};
use std::collections::HashSet;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Retained<ProtocolObject<dyn MTLBuffer>>>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<Retained<ProtocolObject<dyn MTLSamplerState>>>,
    pub pipelines: Slots<Pipeline>,
    pub renders: Slots<Render>,
    pub tiles: Slots<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
    pub depths: Slots<Retained<ProtocolObject<dyn MTLDepthStencilState>>>,
    pub heaps: Slots<(Retained<ProtocolObject<dyn MTLHeap>>, u32)>,
    pub indirects: Slots<Indirect>,
    pub accels: Slots<Accel>,
    pub rays: Slots<u32>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
    pub attachments: Slots<()>,
    pub imports: HashSet<u32>,
    pub table: crate::table::Table,
}
