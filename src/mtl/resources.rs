use crate::mtl::buffer::BufferObject;
use crate::mtl::pipeline::Pipeline;
use crate::mtl::query::Query;
use crate::mtl::render::Render;
use crate::mtl::texture::TextureObject;
use crate::slots::Slots;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::{
    MTLDepthStencilState, MTLRenderPipelineState, MTLResidencySet, MTLResourceID, MTLSamplerState,
    MTLSharedEvent,
};

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<BufferObject>,
    pub textures: Slots<TextureObject>,
    pub samplers: Slots<(Retained<ProtocolObject<dyn MTLSamplerState>>, MTLResourceID)>,
    pub pipelines: Slots<Pipeline>,
    pub renders: Slots<Render>,
    pub tiles: Slots<Retained<ProtocolObject<dyn MTLRenderPipelineState>>>,
    pub depths: Slots<Retained<ProtocolObject<dyn MTLDepthStencilState>>>,
    pub caches: Slots<crate::mtl::cache::Cache>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Retained<ProtocolObject<dyn MTLSharedEvent>>>,
    pub indirects: Slots<crate::mtl::indirect::Indirect>,
    pub sets: Slots<Retained<ProtocolObject<dyn MTLResidencySet>>>,
    pub swapchains: Slots<crate::mtl::swapchain::SwapchainObject>,
    pub layouts: Slots<Vec<crate::DescriptorBinding>>,
    pub descriptor_pools: Slots<crate::mtl::descriptor_pool::DescriptorPoolObject>,
    pub descriptor_sets: Slots<crate::mtl::descriptor_set::DescriptorSetObject>,
    pub table: crate::table::Table,
}
