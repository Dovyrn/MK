use crate::{dimension, format, memory};

pub struct TextureInfo {
    pub dimension: u32,
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub layers: u32,
    pub mips: u32,
    pub samples: u32,
    pub usage: u32,
    pub memory: u32,
}

impl Default for TextureInfo {
    fn default() -> Self {
        Self {
            dimension: dimension::D2,
            format: format::NONE,
            width: 1,
            height: 1,
            depth: 1,
            layers: 1,
            mips: 1,
            samples: 1,
            usage: 0,
            memory: memory::DEVICE,
        }
    }
}
