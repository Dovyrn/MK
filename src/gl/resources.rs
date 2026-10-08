use crate::DepthStateInfo;
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
    pub depths: Slots<DepthStateInfo>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Rc<RefCell<Fences>>>,
    pub pools: Slots<()>,
    pub table: crate::table::Table,
}
