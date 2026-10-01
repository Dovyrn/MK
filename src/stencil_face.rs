use crate::{compare, stencil};

#[derive(Clone, Copy)]
pub struct StencilFace {
    pub fail: u32,
    pub depth_fail: u32,
    pub pass: u32,
    pub compare: u32,
    pub read: u32,
    pub write: u32,
}

impl Default for StencilFace {
    fn default() -> Self {
        Self {
            fail: stencil::KEEP,
            depth_fail: stencil::KEEP,
            pass: stencil::KEEP,
            compare: compare::ALWAYS,
            read: 0xff,
            write: 0xff,
        }
    }
}
