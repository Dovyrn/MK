use crate::DepthInfo;
use crate::slots::Slots;
use crate::vk::accel::Accel;
use crate::vk::buffer::Buffer;
use crate::vk::heap::Heap;
use crate::vk::texture::Texture;
use ash::vk;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Buffer>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<vk::Sampler>,
    pub pipelines: Slots<vk::Pipeline>,
    pub renders: Slots<(vk::Pipeline, vk::PrimitiveTopology)>,
    pub depths: Slots<DepthInfo>,
    pub heaps: Slots<Heap>,
    pub accels: Slots<Accel>,
    pub layouts: Slots<(u64, vk::ShaderStageFlags)>,
    pub rays: Slots<(vk::Pipeline, u32)>,
    pub queries: Slots<(vk::QueryPool, vk::QueryControlFlags, u64, u32)>,
    pub events: Slots<(vk::Semaphore, bool)>,
}
