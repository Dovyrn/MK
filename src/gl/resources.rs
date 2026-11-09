use crate::DepthStencilStateInfo;
use crate::gl::buffer::BufferObject;
use crate::gl::fences::Fences;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::texture::TextureObject;
use crate::slots::Slots;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<BufferObject>,
    pub textures: Slots<TextureObject>,
    pub samplers: Slots<glow::Sampler>,
    pub pipelines: Slots<Program>,
    pub renders: Slots<Pipeline>,
    pub depths: Slots<DepthStencilStateInfo>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Rc<RefCell<Fences>>>,
    pub pools: Slots<()>,
    pub sets: Slots<()>,
    pub layouts: Slots<Vec<crate::DescriptorBinding>>,
    pub descriptor_pools: Slots<crate::gl::descriptor_pool::DescriptorPoolObject>,
    pub descriptor_sets: Slots<crate::gl::descriptor_set::DescriptorSetObject>,
    pub table: crate::table::Table,
    pub framebuffers: RefCell<Vec<(Vec<u32>, glow::Framebuffer)>>,
}
