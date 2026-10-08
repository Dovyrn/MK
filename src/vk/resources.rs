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
    pub pipelines: Slots<vk::Pipeline>,
    pub renders: Slots<(vk::Pipeline, Option<vk::PrimitiveTopology>)>,
    pub depths: Slots<DepthStateInfo>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Semaphore>,
    pub sets: Slots<()>,
    pub table: crate::table::Table,
}
