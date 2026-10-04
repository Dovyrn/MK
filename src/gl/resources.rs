use crate::DepthInfo;
use crate::gl::buffer::Buffer;
use crate::gl::fences::Fences;
use crate::gl::pipeline::Pipeline;
use crate::gl::program::Program;
use crate::gl::query::Query;
use crate::gl::texture::Texture;
use crate::slots::Slots;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub struct Resources {
    pub buffers: Slots<Buffer>,
    pub textures: Slots<Texture>,
    pub samplers: Slots<glow::Sampler>,
    pub pipelines: Slots<Program>,
    pub renders: Slots<Pipeline>,
    pub depths: Slots<DepthInfo>,
    pub queries: Slots<Query>,
    pub timelines: Slots<Rc<RefCell<Fences>>>,
    pub indirects: Slots<u32>,
    pub table: crate::table::Table,
}
