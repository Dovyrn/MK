use crate::DepthInfo;
use crate::slots::Slots;
use crate::vk::accel::Accel;
use crate::vk::buffer::Buffer;
use crate::vk::event::Event;
use crate::vk::heap::Heap;
use crate::vk::query::Query;
use crate::vk::texture::Texture;
use ash::vk;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Buffer>,
    pub indirects: Slots<u32>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<vk::Sampler>,
    pub pipelines: Slots<vk::Pipeline>,
    pub renders: Slots<(vk::Pipeline, vk::PrimitiveTopology)>,
    pub depths: Slots<DepthInfo>,
    pub heaps: Slots<Heap>,
    pub accels: Slots<Accel>,
    pub queries: Slots<Query>,
    pub events: Slots<Event>,
    pub table: crate::table::Table,
}
