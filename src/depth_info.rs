use crate::{StencilFace, compare};

#[derive(Clone, Copy)]
pub struct DepthInfo {
    pub compare: u32,
    pub write: bool,
    pub stencil: bool,
    pub front: StencilFace,
    pub back: StencilFace,
}

impl Default for DepthInfo {
    fn default() -> Self {
        Self {
            compare: compare::NONE,
            write: false,
            stencil: false,
            front: StencilFace::default(),
            back: StencilFace::default(),
        }
    }
}
