use crate::{ColorTarget, VertexAttribute, VertexBuffer, fill, format, topology};

pub struct RenderInfo<'a> {
    pub code: &'a [u8],
    pub vertex: &'a str,
    pub fragment: &'a str,
    pub colors: &'a [ColorTarget],
    pub depth: u32,
    pub stencil: u32,
    pub samples: u32,
    pub topology: u32,
    pub fill: u32,
    pub coverage: bool,
    pub views: u32,
    pub buffers: &'a [VertexBuffer],
    pub attributes: &'a [VertexAttribute],
}

impl Default for RenderInfo<'_> {
    fn default() -> Self {
        Self {
            code: &[],
            vertex: "",
            fragment: "",
            colors: &[],
            depth: format::NONE,
            stencil: format::NONE,
            samples: 1,
            topology: topology::TRIANGLE,
            fill: fill::FILL,
            coverage: false,
            views: 0,
            buffers: &[],
            attributes: &[],
        }
    }
}
