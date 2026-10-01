use crate::{blend, format, mask, op};

#[derive(Clone, Copy)]
pub struct ColorTarget {
    pub format: u32,
    pub blend: bool,
    pub src: u32,
    pub dst: u32,
    pub op: u32,
    pub src_alpha: u32,
    pub dst_alpha: u32,
    pub op_alpha: u32,
    pub mask: u32,
}

impl Default for ColorTarget {
    fn default() -> Self {
        Self {
            format: format::NONE,
            blend: false,
            src: blend::ONE,
            dst: blend::ZERO,
            op: op::ADD,
            src_alpha: blend::ONE,
            dst_alpha: blend::ZERO,
            op_alpha: op::ADD,
            mask: mask::ALL,
        }
    }
}
