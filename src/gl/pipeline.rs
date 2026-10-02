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

pub fn uniform(colors: &[ColorTarget]) -> bool {
    let key = |c: &ColorTarget| (c.src, c.dst, c.op, c.src_alpha, c.dst_alpha, c.op_alpha);
    let mut on = colors.iter().filter(|c| c.blend).map(key);
    let first = on.next();
    on.all(|k| Some(k) == first)
}
