use crate::{dimension, format, usage};

pub struct ViewInfo {
    pub dimension: u32,
    pub format: u32,
    pub base_mip: u32,
    pub mips: u32,
    pub base_layer: u32,
    pub layers: u32,
    pub usage: u32,
}

impl Default for ViewInfo {
    fn default() -> Self {
        Self {
            dimension: dimension::D2,
            format: format::NONE,
            base_mip: 0,
            mips: 1,
            base_layer: 0,
            layers: 1,
            usage: usage::SAMPLED,
        }
    }
}
