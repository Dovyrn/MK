use crate::DepthStateInfo;
use crate::slots::Slots;
use crate::vk::buffer::BufferObject;
use crate::vk::query::Query;
use crate::vk::semaphore::Semaphore;
use crate::vk::texture::TextureObject;
use ash::vk;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<BufferObject>,
    pub textures: Slots<TextureObject>,
    pub samplers: Slots<vk::Sampler>,
    pub pipelines: Slots<(vk::Pipeline, vk::PipelineLayout)>,
    pub renders: Slots<(
        vk::Pipeline,
        Option<vk::PrimitiveTopology>,
        vk::PipelineLayout,
    )>,
    pub depths: Slots<DepthStateInfo<'static>>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Semaphore>,
    pub sets: Slots<()>,
    pub indirects: Slots<crate::Buffer>,
    pub swapchains: Slots<crate::vk::swapchain::SwapchainObject>,
    pub layouts: Slots<(vk::DescriptorSetLayout, Vec<crate::DescriptorBinding>)>,
    pub descriptor_pools: Slots<crate::vk::descriptor_pool::DescriptorPoolObject>,
    pub descriptor_sets: Slots<(vk::DescriptorSet, crate::DescriptorPool)>,
    pub table: crate::table::Table,
}
