use crate::DepthInfo;
use crate::slots::Slots;
use crate::vk::accel::Accel;
use crate::vk::buffer::Buffer;
use crate::vk::surface::Surface;
use crate::vk::texture::Texture;
use ash::vk;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Buffer>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<vk::Sampler>,
    pub pipelines: Slots<vk::Pipeline>,
    pub renders: Slots<vk::Pipeline>,
    pub depths: Slots<DepthInfo>,
    pub accels: Slots<Accel>,
    pub queries: Slots<(vk::QueryPool, vk::QueryControlFlags)>,
    pub surfaces: Slots<Surface>,
    pub events: Slots<(vk::Semaphore, bool)>,
}
