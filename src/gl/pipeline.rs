use crate::gl::program::Program;
use crate::{ColorTarget, VertexAttribute, VertexBuffer};

pub struct Pipeline {
    pub program: Program,
    pub vao: glow::VertexArray,
    pub buffers: Vec<VertexBuffer>,
    pub attributes: Vec<VertexAttribute>,
    pub colors: Vec<ColorTarget>,
    pub topology: u32,
    pub fill: u32,
    pub clamp: bool,
    pub coverage: bool,
}
